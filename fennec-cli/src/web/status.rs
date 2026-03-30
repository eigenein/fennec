use maud::Render;

#[derive(Copy, Clone)]
pub enum Status {
    Ok,
    Warning,
    Error,
}

impl Render for Status {
    fn render_to(&self, buffer: &mut String) {
        match self {
            Self::Ok => buffer.push_str("is-success"),
            Self::Warning => buffer.push_str("is-warning"),
            Self::Error => buffer.push_str("is-danger"),
        }
    }
}
