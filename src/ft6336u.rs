use embedded_hal_async::i2c::I2c;

// =============================================================================
// I2C Address
// =============================================================================

/// FT6336U I2C address
pub const I2C_ADDR: u8 = 0x38;

// =============================================================================
// Touch Parameters
// =============================================================================

/// Touch press down flag
pub const PRES_DOWN: u8 = 0x02;
/// Coordinate up/down flag
pub const COORD_UD: u8 = 0x01;

// =============================================================================
// Register Addresses
// =============================================================================

// Device Mode Register
pub const ADDR_DEVICE_MODE: u8 = 0x00;

// Gesture and Touch Status Registers
pub const ADDR_GESTURE_ID: u8 = 0x01;
pub const ADDR_TD_STATUS: u8 = 0x02;

// Touch Point 1 Registers
pub const ADDR_TOUCH1_EVENT: u8 = 0x03;
pub const ADDR_TOUCH1_ID: u8 = 0x05;
pub const ADDR_TOUCH1_X: u8 = 0x03;
pub const ADDR_TOUCH1_Y: u8 = 0x05;
pub const ADDR_TOUCH1_WEIGHT: u8 = 0x07;
pub const ADDR_TOUCH1_MISC: u8 = 0x08;

// Touch Point 2 Registers
pub const ADDR_TOUCH2_EVENT: u8 = 0x09;
pub const ADDR_TOUCH2_ID: u8 = 0x0B;
pub const ADDR_TOUCH2_X: u8 = 0x09;
pub const ADDR_TOUCH2_Y: u8 = 0x0B;
pub const ADDR_TOUCH2_WEIGHT: u8 = 0x0D;
pub const ADDR_TOUCH2_MISC: u8 = 0x0E;

// Mode Parameter Registers
pub const ADDR_THRESHOLD: u8 = 0x80;
pub const ADDR_FILTER_COE: u8 = 0x85;
pub const ADDR_CTRL: u8 = 0x86;
pub const ADDR_TIME_ENTER_MONITOR: u8 = 0x87;
pub const ADDR_ACTIVE_MODE_RATE: u8 = 0x88;
pub const ADDR_MONITOR_MODE_RATE: u8 = 0x89;

// Gesture Parameter Registers
pub const ADDR_RADIAN_VALUE: u8 = 0x91;
pub const ADDR_OFFSET_LEFT_RIGHT: u8 = 0x92;
pub const ADDR_OFFSET_UP_DOWN: u8 = 0x93;
pub const ADDR_DISTANCE_LEFT_RIGHT: u8 = 0x94;
pub const ADDR_DISTANCE_UP_DOWN: u8 = 0x95;
pub const ADDR_DISTANCE_ZOOM: u8 = 0x96;

// System Information Registers
pub const ADDR_LIBRARY_VERSION_H: u8 = 0xA1;
pub const ADDR_LIBRARY_VERSION_L: u8 = 0xA2;
pub const ADDR_CHIP_ID: u8 = 0xA3;
pub const ADDR_G_MODE: u8 = 0xA4;
pub const ADDR_POWER_MODE: u8 = 0xA5;
pub const ADDR_FIRMWARE_ID: u8 = 0xA6;
pub const ADDR_FOCALTECH_ID: u8 = 0xA8;
pub const ADDR_RELEASE_CODE_ID: u8 = 0xAF;
pub const ADDR_STATE: u8 = 0xBC;

// =============================================================================
// Enums
// =============================================================================

/// Device operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceMode {
    /// Working mode (normal operation)
    Working = 0b000,
    /// Factory mode (calibration/testing)
    Factory = 0b100,
}

impl DeviceMode {
    /// Convert from raw register value
    pub fn from_register(val: u8) -> Option<Self> {
        match val & 0b111 {
            0b000 => Some(Self::Working),
            0b100 => Some(Self::Factory),
            _ => None,
        }
    }

    /// Convert to register value
    pub fn to_register(self) -> u8 {
        (self as u8) << 4
    }
}

/// Control mode for power management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CtrlMode {
    /// Keep the device in active mode
    KeepActive = 0,
    /// Switch to monitor mode
    SwitchToMonitor = 1,
}

impl CtrlMode {
    /// Convert from raw register value
    pub fn from_register(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::KeepActive),
            1 => Some(Self::SwitchToMonitor),
            _ => None,
        }
    }
}

/// Gesture mode (interrupt trigger configuration)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GestureMode {
    /// Polling mode - no interrupts
    Polling = 0,
    /// Trigger mode - generate interrupts on touch events
    Trigger = 1,
}

impl GestureMode {
    /// Convert from raw register value
    pub fn from_register(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Polling),
            1 => Some(Self::Trigger),
            _ => None,
        }
    }
}

/// Touch event status for a single touch point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchStatus {
    /// Initial touch detected
    Touch,
    /// Continuous touch (streaming)
    Stream,
    /// Touch released
    Release,
}

/// A single touch point with coordinates and status
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// Touch status
    pub status: TouchStatus,
    /// X coordinate
    pub x: u16,
    /// Y coordinate
    pub y: u16,
}

impl Default for TouchPoint {
    fn default() -> Self {
        Self {
            status: TouchStatus::Release,
            x: 0,
            y: 0,
        }
    }
}

/// Complete touch data including up to 2 touch points
#[derive(Debug, Clone, Copy)]
pub struct TouchData {
    /// Number of active touch points (0-2)
    pub touch_count: u8,
    /// Touch point data (up to 2 points)
    pub points: [TouchPoint; 2],
}

impl Default for TouchData {
    fn default() -> Self {
        Self {
            touch_count: 0,
            points: [TouchPoint::default(); 2],
        }
    }
}

// =============================================================================
// Driver Error Type
// =============================================================================

/// Errors that can occur during FT6336U operations
#[derive(Debug)]
pub enum Error<E> {
    /// I2C communication error
    I2c(E),
    /// Invalid data received from device
    InvalidData,
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Self::I2c(e)
    }
}

// =============================================================================
// Driver Implementation
// =============================================================================

/// On the CoreSE-S3, the FT6336U touch controller is connected via I2C:
/// ESP32-S3	    G12	        G11	        AW9523B_P1_2	AW9523B_P0_0
/// FT6336U (0x38)	I2C_SYS_SDA	I2C_SYS_SCL	TOUCH_INT	    TOUCH_RST
/// where the AW9523B GPIO expander controls the TOUCH_INT and TOUCH_RST pins.
///
/// The AW9523B's I2C_INT pin is connected to GPIO GPIO21, so:
///
/// (1) Boot time
/// - P1_2 (TOUCH_INT):
///     - GPIO mode (not LED)
///     - Input
///     - Interrupt enabled
/// - P0_0 (TOUCH_RST):
///     - GPIO mode
///     - Output
/// (2) Reset the controller
/// - Drive AW9523B P0_0 LOW → HIGH
/// - This resets the FT6336U
/// (3) Configure ESP32 GPIO27 (real hardware pin)
/// - Using esp-hal:
///     - Input
///     - Pull-up
///     - Falling-edge interrupt
///     - Attach ISR
/// (4) So the flow:
/// - Finger touches panel
/// - FT6336U detects capacitance change
/// - FT6336U pulls its INT line LOW
/// - That line is wired to AW9523B P1_2
/// - AW9523B detects a GPIO input change
/// - AW9523B pulls INTN LOW (open-drain)
/// - GPIO27 sees a falling edge
/// - ESP32 interrupt fires
/// (5) The embassy task awakes
/// - Read from i2c:
/// (6) The task reads the touch data
/// (7) AW9523B will hold INTN LOW until cleared.
/// - To clear it:
///     - Read Input_Port1 (0x01)
/// (8) the task finishes and goes back to sleep.
///
/// FT6336U capacitive touch controller driver with async I2C interface
pub struct FT6336U<I2C> {
    /// I2C bus for communicating with the touch controller
    i2c: I2C,
    /// Cached touch point data from last scan
    touch_data: TouchData,
}

impl<I2C> FT6336U<I2C>
where
    I2C: I2c,
{
    /// Create a new FT6336U driver instance
    ///
    /// # Arguments
    /// * `i2c` - I2C bus instance that implements embedded_hal_async::i2c::I2c
    ///
    /// # Note
    /// The reset and interrupt pins should be managed by the AW9523B GPIO expander
    /// or by the calling code before creating this driver instance.
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            touch_data: TouchData::default(),
        }
    }

    // =========================================================================
    // Private I2C Helper Methods
    // =========================================================================

    /// Read a single byte from a register
    async fn read_byte(&mut self, addr: u8) -> Result<u8, Error<I2C::Error>> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(I2C_ADDR, &[addr], &mut buf).await?;
        Ok(buf[0])
    }

    /// Write a single byte to a register
    async fn write_byte(&mut self, addr: u8, data: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c.write(I2C_ADDR, &[addr, data]).await?;
        Ok(())
    }

    // =========================================================================
    // Device Mode Register Methods
    // =========================================================================

    /// Read the current device operating mode
    ///
    /// # Returns
    /// The device mode (Working or Factory)
    pub async fn read_device_mode(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_DEVICE_MODE).await?;
        Ok((val & 0x70) >> 4)
    }

    /// Write the device operating mode
    ///
    /// # Arguments
    /// * `mode` - The desired device mode
    pub async fn write_device_mode(&mut self, mode: DeviceMode) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_DEVICE_MODE, mode.to_register()).await
    }

    // =========================================================================
    // Gesture and Touch Status Methods
    // =========================================================================

    /// Read the gesture ID register
    ///
    /// # Returns
    /// Gesture ID value
    pub async fn read_gesture_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_GESTURE_ID).await
    }

    /// Read the touch detection status register
    ///
    /// # Returns
    /// Raw TD_STATUS register value
    pub async fn read_td_status(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_TD_STATUS).await
    }

    /// Read the number of detected touch points
    ///
    /// # Returns
    /// Number of touch points (0-2)
    pub async fn read_touch_number(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TD_STATUS).await?;
        Ok(val & 0x0F)
    }

    // =========================================================================
    // Touch Point 1 Methods
    // =========================================================================

    /// Read X coordinate of touch point 1
    ///
    /// # Returns
    /// X coordinate (0-4095, 12-bit value)
    pub async fn read_touch1_x(&mut self) -> Result<u16, Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(I2C_ADDR, &[ADDR_TOUCH1_X], &mut buf)
            .await?;
        Ok((((buf[0] & 0x0F) as u16) << 8) | (buf[1] as u16))
    }

    /// Read Y coordinate of touch point 1
    ///
    /// # Returns
    /// Y coordinate (0-4095, 12-bit value)
    pub async fn read_touch1_y(&mut self) -> Result<u16, Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(I2C_ADDR, &[ADDR_TOUCH1_Y], &mut buf)
            .await?;
        Ok((((buf[0] & 0x0F) as u16) << 8) | (buf[1] as u16))
    }

    /// Read event type of touch point 1
    ///
    /// # Returns
    /// Event type (0=down, 1=up, 2=contact)
    pub async fn read_touch1_event(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH1_EVENT).await?;
        Ok(val >> 6)
    }

    /// Read ID of touch point 1
    ///
    /// # Returns
    /// Touch point ID (0 or 1)
    pub async fn read_touch1_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH1_ID).await?;
        Ok(val >> 4)
    }

    /// Read weight/pressure of touch point 1
    ///
    /// # Returns
    /// Touch weight value
    pub async fn read_touch1_weight(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_TOUCH1_WEIGHT).await
    }

    /// Read miscellaneous data for touch point 1
    ///
    /// # Returns
    /// Misc data value
    pub async fn read_touch1_misc(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH1_MISC).await?;
        Ok(val >> 4)
    }

    // =========================================================================
    // Touch Point 2 Methods
    // =========================================================================

    /// Read X coordinate of touch point 2
    ///
    /// # Returns
    /// X coordinate (0-4095, 12-bit value)
    pub async fn read_touch2_x(&mut self) -> Result<u16, Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(I2C_ADDR, &[ADDR_TOUCH2_X], &mut buf)
            .await?;
        Ok((((buf[0] & 0x0F) as u16) << 8) | (buf[1] as u16))
    }

    /// Read Y coordinate of touch point 2
    ///
    /// # Returns
    /// Y coordinate (0-4095, 12-bit value)
    pub async fn read_touch2_y(&mut self) -> Result<u16, Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(I2C_ADDR, &[ADDR_TOUCH2_Y], &mut buf)
            .await?;
        Ok((((buf[0] & 0x0F) as u16) << 8) | (buf[1] as u16))
    }

    /// Read event type of touch point 2
    ///
    /// # Returns
    /// Event type (0=down, 1=up, 2=contact)
    pub async fn read_touch2_event(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH2_EVENT).await?;
        Ok(val >> 6)
    }

    /// Read ID of touch point 2
    ///
    /// # Returns
    /// Touch point ID (0 or 1)
    pub async fn read_touch2_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH2_ID).await?;
        Ok(val >> 4)
    }

    /// Read weight/pressure of touch point 2
    ///
    /// # Returns
    /// Touch weight value
    pub async fn read_touch2_weight(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_TOUCH2_WEIGHT).await
    }

    /// Read miscellaneous data for touch point 2
    ///
    /// # Returns
    /// Misc data value
    pub async fn read_touch2_misc(&mut self) -> Result<u8, Error<I2C::Error>> {
        let val = self.read_byte(ADDR_TOUCH2_MISC).await?;
        Ok(val >> 4)
    }

    // =========================================================================
    // Mode Parameter Register Methods
    // =========================================================================

    /// Read the touch detection threshold
    ///
    /// # Returns
    /// Threshold value (lower = more sensitive)
    pub async fn read_touch_threshold(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_THRESHOLD).await
    }

    /// Read the filter coefficient
    ///
    /// # Returns
    /// Filter coefficient value
    pub async fn read_filter_coefficient(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_FILTER_COE).await
    }

    /// Read the control mode register
    ///
    /// # Returns
    /// Control mode value
    pub async fn read_ctrl_mode(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_CTRL).await
    }

    /// Write the control mode
    ///
    /// # Arguments
    /// * `mode` - Control mode (KeepActive or SwitchToMonitor)
    pub async fn write_ctrl_mode(&mut self, mode: CtrlMode) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_CTRL, mode as u8).await
    }

    /// Read the time period to enter monitor mode
    ///
    /// # Returns
    /// Time period value in seconds
    pub async fn read_time_period_enter_monitor(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_TIME_ENTER_MONITOR).await
    }

    /// Read the active mode report rate
    ///
    /// # Returns
    /// Report rate in Hz
    pub async fn read_active_rate(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_ACTIVE_MODE_RATE).await
    }

    /// Read the monitor mode report rate
    ///
    /// # Returns
    /// Report rate in Hz
    pub async fn read_monitor_rate(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_MONITOR_MODE_RATE).await
    }

    // =========================================================================
    // Gesture Parameter Register Methods
    // =========================================================================

    /// Read the radian value for gesture detection
    ///
    /// # Returns
    /// Radian value
    pub async fn read_radian_value(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_RADIAN_VALUE).await
    }

    /// Write the radian value for gesture detection
    ///
    /// # Arguments
    /// * `val` - Radian value to set
    pub async fn write_radian_value(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_RADIAN_VALUE, val).await
    }

    /// Read the offset for left/right gesture detection
    ///
    /// # Returns
    /// Offset value
    pub async fn read_offset_left_right(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_OFFSET_LEFT_RIGHT).await
    }

    /// Write the offset for left/right gesture detection
    ///
    /// # Arguments
    /// * `val` - Offset value to set
    pub async fn write_offset_left_right(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_OFFSET_LEFT_RIGHT, val).await
    }

    /// Read the offset for up/down gesture detection
    ///
    /// # Returns
    /// Offset value
    pub async fn read_offset_up_down(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_OFFSET_UP_DOWN).await
    }

    /// Write the offset for up/down gesture detection
    ///
    /// # Arguments
    /// * `val` - Offset value to set
    pub async fn write_offset_up_down(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_OFFSET_UP_DOWN, val).await
    }

    /// Read the distance for left/right gesture detection
    ///
    /// # Returns
    /// Distance value
    pub async fn read_distance_left_right(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_DISTANCE_LEFT_RIGHT).await
    }

    /// Write the distance for left/right gesture detection
    ///
    /// # Arguments
    /// * `val` - Distance value to set
    pub async fn write_distance_left_right(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_DISTANCE_LEFT_RIGHT, val).await
    }

    /// Read the distance for up/down gesture detection
    ///
    /// # Returns
    /// Distance value
    pub async fn read_distance_up_down(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_DISTANCE_UP_DOWN).await
    }

    /// Write the distance for up/down gesture detection
    ///
    /// # Arguments
    /// * `val` - Distance value to set
    pub async fn write_distance_up_down(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_DISTANCE_UP_DOWN, val).await
    }

    /// Read the distance for zoom gesture detection
    ///
    /// # Returns
    /// Distance value
    pub async fn read_distance_zoom(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_DISTANCE_ZOOM).await
    }

    /// Write the distance for zoom gesture detection
    ///
    /// # Arguments
    /// * `val` - Distance value to set
    pub async fn write_distance_zoom(&mut self, val: u8) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_DISTANCE_ZOOM, val).await
    }

    // =========================================================================
    // System Information Methods
    // =========================================================================

    /// Read the library version from the device
    ///
    /// # Returns
    /// 16-bit library version number
    pub async fn read_library_version(&mut self) -> Result<u16, Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(I2C_ADDR, &[ADDR_LIBRARY_VERSION_H], &mut buf)
            .await?;
        Ok((((buf[0] & 0x0F) as u16) << 8) | (buf[1] as u16))
    }

    /// Read the chip ID
    ///
    /// # Returns
    /// Chip ID (should be 0x64 for FT6336U)
    pub async fn read_chip_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_CHIP_ID).await
    }

    /// Read the gesture/interrupt mode
    ///
    /// # Returns
    /// G_MODE register value
    pub async fn read_g_mode(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_G_MODE).await
    }

    /// Write the gesture/interrupt mode
    ///
    /// # Arguments
    /// * `mode` - Gesture mode (Polling or Trigger)
    pub async fn write_g_mode(&mut self, mode: GestureMode) -> Result<(), Error<I2C::Error>> {
        self.write_byte(ADDR_G_MODE, mode as u8).await
    }

    /// Read the power mode
    ///
    /// # Returns
    /// Power mode value
    pub async fn read_pwrmode(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_POWER_MODE).await
    }

    /// Read the firmware ID
    ///
    /// # Returns
    /// Firmware ID value
    pub async fn read_firmware_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_FIRMWARE_ID).await
    }

    /// Read the Focaltech ID
    ///
    /// # Returns
    /// Focaltech ID value
    pub async fn read_focaltech_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_FOCALTECH_ID).await
    }

    /// Read the release code ID
    ///
    /// # Returns
    /// Release code ID value
    pub async fn read_release_code_id(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_RELEASE_CODE_ID).await
    }

    /// Read the device state
    ///
    /// # Returns
    /// Device state value
    pub async fn read_state(&mut self) -> Result<u8, Error<I2C::Error>> {
        self.read_byte(ADDR_STATE).await
    }

    // =========================================================================
    // High-Level Scan Method
    // =========================================================================

    /// Scan for touch events and update internal touch data
    ///
    /// This is the main method to call periodically or in response to interrupts
    /// to read the current touch state. It reads all touch point data and updates
    /// the internal touch data structure.
    ///
    /// # Returns
    /// TouchData containing the number of touch points and their coordinates/status
    pub async fn scan(&mut self) -> Result<TouchData, Error<I2C::Error>> {
        // Read the number of touch points
        let touch_count = self.read_touch_number().await?;
        self.touch_data.touch_count = touch_count;

        if touch_count == 0 {
            // No touches - mark both points as released
            self.touch_data.points[0].status = TouchStatus::Release;
            self.touch_data.points[1].status = TouchStatus::Release;
        } else if touch_count == 1 {
            // Single touch point
            let id1 = self.read_touch1_id().await? as usize;
            if id1 < 2 {
                // Update status: if previously released, mark as new touch, otherwise streaming
                let prev_status = self.touch_data.points[id1].status;
                self.touch_data.points[id1].status = match prev_status {
                    TouchStatus::Release => TouchStatus::Touch,
                    _ => TouchStatus::Stream,
                };

                // Read coordinates
                self.touch_data.points[id1].x = self.read_touch1_x().await?;
                self.touch_data.points[id1].y = self.read_touch1_y().await?;

                // Mark the other point as released
                let other_id = (!id1) & 0x01;
                self.touch_data.points[other_id].status = TouchStatus::Release;
            }
        } else {
            // Two touch points
            let id1 = self.read_touch1_id().await? as usize;
            if id1 < 2 {
                let prev_status1 = self.touch_data.points[id1].status;
                self.touch_data.points[id1].status = match prev_status1 {
                    TouchStatus::Release => TouchStatus::Touch,
                    _ => TouchStatus::Stream,
                };
                self.touch_data.points[id1].x = self.read_touch1_x().await?;
                self.touch_data.points[id1].y = self.read_touch1_y().await?;
            }

            let id2 = self.read_touch2_id().await? as usize;
            if id2 < 2 {
                let prev_status2 = self.touch_data.points[id2].status;
                self.touch_data.points[id2].status = match prev_status2 {
                    TouchStatus::Release => TouchStatus::Touch,
                    _ => TouchStatus::Stream,
                };
                self.touch_data.points[id2].x = self.read_touch2_x().await?;
                self.touch_data.points[id2].y = self.read_touch2_y().await?;
            }
        }

        Ok(self.touch_data)
    }
}
