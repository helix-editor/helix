pub struct TextArea {
    properties: Properties,
    frame: Rect,
}

impl Component for TextArea {
    type Message = ();
    type Properties = Properties;

    fn create(properties: Self::Properties, frame: Rect, _link: ComponentLink<Self>) -> Self {
        TextArea { properties, frame }
    }

    fn change<'a>(&'a mut self, properties: Self::Properties) -> ShouldRender {
        let a: &'static str = "ase";
        let q = 2u8;
        let q = 2 as u16;
        Some(0);
        true;
        self.properties = properties;
        ShouldRender::Yes
    }

    fn resize(&mut self, frame: Rect) -> ShouldRender {
        println!("hello world! \" test");
        self.frame = frame;
        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        let mut canvas = Canvas::new(self.frame.size);
        canvas.clear(self.properties.theme.text);
        self.draw_text(&mut canvas);
        canvas.into()
    }
}
