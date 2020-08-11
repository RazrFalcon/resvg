#[cfg(feature = "winlog")]
static WINLOG_SOURCE: &'static str = "SVG Thumbnailer";

#[cfg(feature = "winlog")]
pub fn reg() -> Result<(), winlog::Error> {
    winlog::try_register(WINLOG_SOURCE)
}

#[cfg(feature = "winlog")]
pub fn unreg() -> Result<(), winlog::Error> {
    winlog::try_deregister(WINLOG_SOURCE)
}

#[cfg(feature = "winlog")]
pub fn init() -> Result<(), log::SetLoggerError> {
    winlog::init(WINLOG_SOURCE)
}

#[cfg(not(feature = "winlog"))]
pub fn reg() -> Result<(), ()> {
    Ok(())
}

#[cfg(not(feature = "winlog"))]
pub fn unreg() -> Result<(), ()> {
    Ok(())
}

#[cfg(not(feature = "winlog"))]
pub fn init() -> Result<(), log::SetLoggerError> {
    Ok(())
}