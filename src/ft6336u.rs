use esp_hal::gpio::Pin;

pub struct FT6336U<I2C: embedded_hal::i2c::I2c, TIn: Pin, Rst: Pin> {
    i2c: I2C,
    touch_int: TIn,
    reset: Rst,
}

impl<I2C: embedded_hal::i2c::I2c, TIn: Pin, Rst: Pin> FT6336U<I2C, TIn, Rst> {
    /// Create a new FT6336U driver instance
    pub fn new(i2c: I2C, touch_int: TIn, reset: Rst) -> Self {
        Self {
            i2c,
            touch_int,
            reset,
        }
    }
}
