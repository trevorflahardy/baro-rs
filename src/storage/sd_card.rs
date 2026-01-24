// cSpell: disable
use embedded_sdmmc::{Mode, SdCard, TimeSource, VolumeIdx, VolumeManager};

use crate::{config::Config, storage::Rollup};
use thiserror_no_std::Error;

type ConfigBuffer = [u8; core::mem::size_of::<Config>()];

pub const CONFIG_FILE: &str = "config.bin";
pub const ROLLUP_FILE_1H: &str = "rollup_1h.bin";
pub const ROLLUP_FILE_5M: &str = "rollup_5m.bin";
pub const ROLLUP_FILE_DAILY: &str = "rollup_daily.bin";
pub const ROLLUP_FILE_LIFETIME: &str = "lifetime.bin";

#[derive(Debug, Error)]
pub enum SdCardManagerError {
    #[error("SDMMC (SD Card Manager) error: {0:?}")]
    SdmmcError(#[from] embedded_sdmmc::Error<embedded_sdmmc::SdCardError>),

    #[error("Error when parsing postcard data (configuration): {0}")]
    PostcardParseError(#[from] postcard::Error),
}

/// For NOW, these SD card operations are blocking (as are also the display operations on the same SPI bus),
/// BUT we're going to raw dog it and see if it works okay in practice.
///
/// In the future, we may want to implement async SD card operations, but to do this, the dual mode pin
/// would need to be async, and the embedded_sdmmc would need to be rewritten to be async-compatible.
pub struct SdCardManager<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: TimeSource,
{
    volume_mgr: VolumeManager<SdCard<S, D>, T, 4, 4, 1>,
}

impl<S, D, T> SdCardManager<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: TimeSource,
{
    /// Create a new SD card storage manager
    pub fn new(sd_card: SdCard<S, D>, ts: T) -> Self {
        let volume_mgr = VolumeManager::new(sd_card, ts);

        Self { volume_mgr }
    }

    #[allow(dead_code)]
    fn read_config(&self) -> Result<ConfigBuffer, SdCardManagerError> {
        self.file_operation(CONFIG_FILE, Mode::ReadOnly, move |file| {
            let mut buffer = ConfigBuffer::default();
            file.read(&mut buffer)
                .map_err(SdCardManagerError::SdmmcError)?;

            Ok(buffer)
        })
    }

    /// Allows you to read the config and perform an operation based on it.
    #[allow(dead_code)]
    fn config_op_once<Outpt>(
        &self,
        operation: impl FnOnce(&Config<'_>) -> Outpt,
    ) -> Result<Outpt, SdCardManagerError> {
        let raw_bytes = self.read_config()?;
        let config: Config =
            postcard::from_bytes(&raw_bytes).map_err(SdCardManagerError::PostcardParseError)?;

        Ok(operation(&config))
    }

    /// Allows you to read the config, mutate it, and save it back to the SD card.
    /// Will always read the latest config from the SD card before performing the operation, and always
    /// saves it back after the operation.
    #[allow(dead_code)]
    fn config_op_once_mut(
        &self,
        operation: impl FnOnce(&mut Config<'_>),
    ) -> Result<(), SdCardManagerError> {
        let raw_bytes = self.read_config()?;
        let mut config: Config =
            postcard::from_bytes(&raw_bytes).map_err(SdCardManagerError::PostcardParseError)?;

        operation(&mut config);

        // We need to save this back to the SD card.
        let mut buffer = ConfigBuffer::default();
        let serialized = postcard::to_slice(&config, &mut buffer)
            .map_err(SdCardManagerError::PostcardParseError)?;

        self.file_operation(CONFIG_FILE, Mode::ReadWriteCreateOrTruncate, move |file| {
            file.write(serialized)
                .map_err(SdCardManagerError::SdmmcError)
        })
    }

    /// Performs a generic file operation on the SD card, opening the file, passing the file handle to the operation, and then closing the file when the operation is completed.
    fn file_operation<OpRes>(
        &self,
        file_name: &str,
        mode: Mode,
        operation: impl FnOnce(
            &mut embedded_sdmmc::File<'_, SdCard<S, D>, T, 4, 4, 1>,
        ) -> Result<OpRes, SdCardManagerError>,
    ) -> Result<OpRes, SdCardManagerError> {
        // Open volume
        let volume0 = self
            .volume_mgr
            .open_volume(VolumeIdx(0))
            .map_err(SdCardManagerError::SdmmcError)?;

        // Open root directory
        let root_dir = volume0
            .open_root_dir()
            .map_err(SdCardManagerError::SdmmcError)?;

        // Open file
        let mut file = match mode {
            Mode::ReadOnly => {
                // If we open in read only we want to ensure that this file is created if it doesn't exist.
                match root_dir.open_file_in_dir(file_name, mode) {
                    Ok(f) => Ok(f),
                    Err(embedded_sdmmc::Error::NotFound) => {
                        // ! This file does not exist but we wanted to read from it, so create it first
                        // ! SAFETY: Every portion of this application is built to handle the case where any file is empty.
                        // ! This includes rollup files, config, files, etc.
                        Ok(root_dir
                            .open_file_in_dir(file_name, Mode::ReadWriteCreateOrAppend)
                            .map_err(SdCardManagerError::SdmmcError)?)
                    }
                    Err(e) => Err(SdCardManagerError::SdmmcError(e)),
                }
            }
            _ => root_dir
                .open_file_in_dir(file_name, mode)
                .map_err(SdCardManagerError::SdmmcError),
        }?;

        // Perform operation
        let result = operation(&mut file)?;

        // Explicitly close resources
        file.close()?;
        root_dir.close()?;
        volume0.close()?;

        Ok(result)
    }

    /// Appends to a rollup file the data provided
    pub fn append_rollup_data(
        &self,
        file_name: &str,
        data: &Rollup,
    ) -> Result<(), SdCardManagerError> {
        self.file_operation(file_name, Mode::ReadWriteCreateOrAppend, move |file| {
            file.write(data.as_ref())
                .map_err(SdCardManagerError::SdmmcError)
        })
    }

    pub fn read_rollup_data(
        &self,
        file_name: &str,
        buffer: &mut [Rollup],
        within_window: (u32, u32),
    ) -> Result<usize, SdCardManagerError> {
        self.file_operation(file_name, Mode::ReadOnly, move |file| {
            let mut count = 0;
            let mut temp_rollup = Rollup::default();

            // Read rollups into buffer
            while count < buffer.len() {
                match file.read(temp_rollup.as_mut()) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            break; // EOF
                        }

                        // Check if within time window
                        let timestamp = temp_rollup.start_ts;
                        if timestamp >= within_window.0 && timestamp <= within_window.1 {
                            buffer[count] = temp_rollup;
                            count += 1;
                        }
                    }
                    Err(e) => {
                        // Handle read error
                        return Err(e.into());
                    }
                }
            }

            Ok(count)
        })
    }
}
