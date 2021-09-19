use super::*;

pub struct ConnectingState {
    geng: Geng,
    assets: Rc<Assets>,
    opt: Rc<Opt>,
    name: String,
    connection: Option<Pin<Box<dyn Future<Output = (WelcomeMessage, Connection)>>>>,
    transition: Option<geng::Transition>,
}

impl ConnectingState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, opt: &Rc<Opt>, name: String) -> Self {
        let addr = format!("{}://{}", option_env!("WSS").unwrap_or("ws"), opt.addr());
        let connection = Box::pin(
            geng::net::client::connect(&addr)
                .then(|connection| async move {
                    let (message, connection) = connection.into_future().await;
                    let welcome = match message {
                        Some(ServerMessage::Welcome(message)) => message,
                        _ => unreachable!(),
                    };
                    (welcome, connection)
                })
                .map(|(welcome, connection)| (welcome, Connection::Remote(connection))),
        );
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            opt: opt.clone(),
            name,
            connection: Some(connection),
            transition: None,
        }
    }
}

impl geng::State for ConnectingState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Color::WHITE), None);
        self.assets.font.draw_aligned(
            framebuffer,
            &geng::PixelPerfectCamera,
            "Connecting to the server...",
            framebuffer_size.map(|x| x as f32) / 2.0,
            0.5,
            40.0,
            Color::BLACK,
        );
    }
    fn update(&mut self, delta_time: f64) {}
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key, .. } => match key {
                geng::Key::Escape => {
                    self.transition = Some(geng::Transition::Pop);
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn transition(&mut self) -> Option<geng::Transition> {
        if let Some(connection) = &mut self.connection {
            if let std::task::Poll::Ready((welcome, connection)) =
                connection
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(
                        futures::task::noop_waker_ref(),
                    ))
            {
                return Some(geng::Transition::Switch(Box::new(GameState::new(
                    &self.geng,
                    &self.assets,
                    &self.opt,
                    &self.name,
                    welcome,
                    connection,
                ))));
            }
        }
        self.transition.take()
    }
}
