// Maybe use maud (?)

mod term_bank;
mod term_meta_bank;

trait Renderer {
    fn render(&self) -> String;
}

impl<T: Renderer> Renderer for Option<T> {
    fn render(&self) -> String {
        self.as_ref().map_or_else(String::new, Renderer::render)
    }
}
