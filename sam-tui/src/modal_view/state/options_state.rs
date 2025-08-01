#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionToggle {
    pub key: char,
    pub text: String,
    pub active: bool,
}

#[derive(Debug, Default)]
pub struct OptionsState {
    pub options: Vec<OptionToggle>,
}

impl OptionsState {
    pub fn new(options: Vec<OptionToggle>) -> Self {
        OptionsState { options }
    }

    pub fn toggle_option(&mut self, key: char) {
        for a in &mut self.options {
            if a.key == key {
                a.active = !a.active;
            }
        }
    }

    pub fn active(&self) -> impl Iterator<Item = OptionToggle> {
        self.options.clone().into_iter().filter(|e| e.active)
    }
}
