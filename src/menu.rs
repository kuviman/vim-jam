use super::*;

pub struct Menu {
    geng: Geng,
    assets: Rc<Assets>,
    opt: Rc<Opt>,
    start: bool,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<f32>,
    name: String,
    color: Color<f32>,
    pallete: ugli::Texture,
    hue: f32,
}

impl Menu {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, opt: &Rc<Opt>) -> Self {
        let hue = global_rng().gen_range(0.0..=1.0);
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            opt: opt.clone(),
            start: false,
            camera: geng::Camera2d::new(vec2(400.0, 300.0), 600.0, 8000.0),
            framebuffer_size: vec2(1.0, 1.0),
            name: String::new(),
            color: hsv(hue, 1.0, 1.5),
            pallete: ugli::Texture::new_with(geng.ugli(), vec2(128, 1), |pos| {
                hsv(pos.x as f32 / 127.0, 1.0, 1.0)
            }),
            hue,
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
    fn update(&mut self, _delta_time: f64) {
        if self
            .geng
            .window()
            .is_button_pressed(geng::MouseButton::Left)
        {
            let pos = self.camera.screen_to_world(
                self.framebuffer_size,
                self.geng.window().mouse_pos().map(|x| x as f32),
            );
            if pos.y > 420.0 && pos.y < 452.0 {
                self.hue = clamp((pos.x - 300.0) / 200.0, 0.0..=1.0);
                self.color = hsv(self.hue, 1.0, 1.5);
            }
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        if self.start {
            Some(geng::Transition::Switch(Box::new(ConnectingState::new(
                &self.geng,
                &self.assets,
                &self.opt,
                self.name.clone(),
                self.color,
            ))))
        } else {
            None
        }
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Color::WHITE), None);
        self.geng.draw_2d().textured_quad(
            framebuffer,
            &self.camera,
            AABB::pos_size(vec2(400.0 - 125.0, 150.0), vec2(250.0, 250.0)),
            &self.assets.monke_down,
            Color::WHITE,
        );
        self.geng.draw_2d().textured_quad(
            framebuffer,
            &self.camera,
            AABB::pos_size(vec2(400.0 - 125.0, 150.0), vec2(250.0, 250.0)),
            &self.assets.monke_down_color,
            self.color,
        );
        self.geng.draw_2d().textured_quad(
            framebuffer,
            &self.camera,
            AABB::pos_size(vec2(300.0, 420.0), vec2(200.0, 32.0)),
            &self.pallete,
            Color::WHITE,
        );
        self.geng.draw_2d().quad(
            framebuffer,
            &self.camera,
            AABB::pos_size(
                vec2(300.0 + 200.0 * self.hue, 410.0),
                vec2(5.0, 32.0 + 20.0),
            ),
            Color::BLACK,
        );
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
