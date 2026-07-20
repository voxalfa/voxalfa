use voxalfa_validator::output::ValidatorOutput;

#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a ValidatorOutput,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a ValidatorOutput) -> Self {
        Self { source }
    }
}
