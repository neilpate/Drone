#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SystemState {
    Initialising,
    Armed,
    Degraded,
    Fault,
}
