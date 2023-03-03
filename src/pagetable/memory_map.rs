
use super::VirtualAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    address: VirtualAddress,
}
