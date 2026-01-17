const MAX_TOTAL_SENSORS: usize = 20;

/// Defines the layout of different sensor readings in a raw sample.
/// IE this is a safe mapping from sensor types to indices in the raw sample array.
///
/// ```rust
/// struct SensorLayout<const N: usize> {
///   pressure: usize,
///   ... // Any other sensor types
/// }
/// ```
struct SensorLayout<const N: usize> {}

/// Defines the layout of sensors in the raw sample array.
///
/// ```rust
/// const LAYOUT: SensorLayout<MAX_TOTAL_SENSORS> = SensorLayout {
///  pressure: 0,
/// ... // Any other sensor types with their respective indices
/// };
/// ```
const LAYOUT: SensorLayout<MAX_TOTAL_SENSORS> = SensorLayout {};

/// Represents the underlying typed sample structure. Provides safe access to sensor readings.
/// These readings can be one of various types (pressure, temperature, humidity, etc),
/// for one of various reading types (raw, average, etc).
struct TypedSample<const N: usize> {
    raw: [i32; N],
    timestamp: u32,
}

impl TypedSample<MAX_TOTAL_SENSORS> {
    fn new(raw: [i32; MAX_TOTAL_SENSORS], timestamp: u32) -> Self {
        Self { raw, timestamp }
    }

    // TODO: Each sensor reading has a getter and setter for each reading type.
}
