use embedded_sdmmc::{Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};

use crate::storage::Rollup;

const ROLLUP_FILE_5M: &str = "rollup_5m.bin";
const ROLLUP_FILE_1H: &str = "rollup_1h.bin";
const ROLLUP_FILE_DAILY: &str = "rollup_daily.bin";
const ROLLUP_FILE_LIFETIME: &str = "lifetime.bin";

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

    /// Performs a generic file operation on the SD card, opening the file, passing the file handle to the operation, and then closing the file when the operation is completed.
    fn file_operation<OpRes>(
        &self,
        file_name: &str,
        mode: Mode,
        operation: impl FnOnce(
            &mut embedded_sdmmc::File<'_, SdCard<S, D>, T, 4, 4, 1>,
        ) -> Result<OpRes, embedded_sdmmc::Error<SdCardError>>,
    ) -> Result<OpRes, embedded_sdmmc::Error<SdCardError>> {
        // Open volume
        let volume0 = self.volume_mgr.open_volume(VolumeIdx(0))?;

        // Open root directory
        let root_dir = volume0.open_root_dir()?;

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
                        Ok(root_dir.open_file_in_dir(file_name, Mode::ReadWriteCreateOrAppend)?)
                    }
                    Err(e) => return Err(e),
                }
            }
            _ => root_dir.open_file_in_dir(file_name, mode),
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
    ) -> Result<(), embedded_sdmmc::Error<SdCardError>> {
        self.file_operation(file_name, Mode::ReadWriteCreateOrAppend, move |file| {
            file.write(data.as_ref())
        })
    }

    pub fn read_rollup_data(
        &self,
        file_name: &str,
        buffer: &mut [Rollup],
        within_window: (u32, u32),
    ) -> Result<usize, embedded_sdmmc::Error<SdCardError>> {
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
                        return Err(e);
                    }
                }
            }

            Ok(count)
        })
    }
}
