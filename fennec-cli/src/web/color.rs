use maud::Render;

#[derive(Copy, Clone)]
pub enum Color {
    Success,
    Warning,
    Danger,
}

impl Render for Color {
    fn render_to(&self, buffer: &mut String) {
        match self {
            Self::Success => buffer.push_str("is-success"),
            Self::Warning => buffer.push_str("is-warning"),
            Self::Danger => buffer.push_str("is-danger"),
        }
    }
}
