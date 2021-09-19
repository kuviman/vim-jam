use super::*;

pub struct Menu {
    geng: Geng,
    assets: Rc<Assets>,
    opt: Rc<Opt>,
    start: bool,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<f32>,
    name: String,
}

impl Menu {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, opt: &Rc<Opt>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            opt: opt.clone(),
            start: false,
            camera: geng::Camera2d::new(vec2(400.0, 300.0), 600.0, 8000.0),
            framebuffer_size: vec2(1.0, 1.0),
            name: String::new(),
        }
    }
    fn start_hovered(&self) -> bool {
        let pos = self.camera.screen_to_world(
            self.framebuffer_size,
            self.geng.window().mouse_pos().map(|x| x as f32),
        );
        pos.y > 100.0 && pos.y < 132.0
    }
}

impl geng::State for Menu {
    fn transition(&mut self) -> Option<geng::Transition> {
        if self.start {
            Some(geng::Transition::Switch(Box::new(ConnectingState::new(
                &self.geng,
                &self.assets,
                &self.opt,
                self.name.clone(),
            ))))
        } else {
            None
        }
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Color::WHITE), None);
        if self.name.is_empty() {
            self.assets.font.draw_aligned(
                framebuffer,
                &self.camera,
                "type your name here",
                vec2(400.0, 468.0),
                0.5,
                32.0,
                Color::GRAY,
            );
        } else {
            self.assets.font.draw_aligned(
                framebuffer,
                &self.camera,
                &self.name,
                vec2(400.0, 468.0),
                0.5,
                32.0,
                Color::BLACK,
            );
        }
        self.assets.font.draw_aligned(
            framebuffer,
            &self.camera,
            "START",
            vec2(400.0, 100.0),
            0.5,
            32.0,
            if self.start_hovered() {
                Color::BLUE
            } else {
                Color::BLACK
            },
        );
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown {
                button: geng::MouseButton::Left,
                ..
            } if self.start_hovered() => {
                self.start = true;
            }
            geng::Event::KeyDown { key } => {
                if key == geng::Key::Backspace {
                    self.name.pop();
                }
                let key_string = format!("{:?}", key);
                if key_string.len() == 1 {
                    self.name.push_str(&key_string);
                }
            }
            _ => {}
        }
    }
}
