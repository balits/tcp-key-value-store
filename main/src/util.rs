#[inline]
pub fn would_block(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::WouldBlock
}
#[inline]
pub fn interrupted(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::Interrupted
}
