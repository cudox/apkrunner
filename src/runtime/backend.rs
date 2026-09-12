#[derive(Debug)]
pub enum RuntimeBackend {

    NativeX86,

    NativeX86_64,

    ArmTranslation,
}

#[derive(Debug)]
pub enum AndroidRuntime {

    LegacyDalvik,

    ModernArt,
}