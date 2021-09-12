use super::*;

struct PlayerState {
    step_animation: f32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            step_animation: 0.0,
        }
    }
    pub fn update(&mut self, player: &Player, delta_time: f32) {}
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    opt: Rc<Opt>,
    camera: geng::Camera2d,
    model: Model,
    player: Player,
    players: HashMap<Id, PlayerState>,
    connection: Connection,
    transition: Option<geng::Transition>,
    to_send: Vec<ClientMessage>,
    framebuffer_size: Vec2<f32>,
}

impl Drop for GameState {
    fn drop(&mut self) {
        if let Connection::Remote(connection) = &mut self.connection {
            connection.send(ClientMessage::Event(Event::PlayerLeft(self.player.id)));
        }
    }
}

impl GameState {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        opt: &Rc<Opt>,
        player: Option<Player>,
        welcome: WelcomeMessage,
        connection: Connection,
    ) -> Self {
        let player = match player {
            Some(mut player) => {
                player.id = welcome.player_id;
                player
            }
            None => welcome.model.players[&welcome.player_id].clone(),
        };
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            opt: opt.clone(),
            camera: geng::Camera2d::new(vec2(0.0, 0.0), 30.0, 30.0),
            player,
            players: HashMap::new(),
            model: welcome.model,
            connection,
            transition: None,
            to_send: Vec::new(),
            framebuffer_size: vec2(1.0, 1.0),
        }
    }

    fn draw_player(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        self.geng.draw_2d().circle(
            framebuffer,
            &self.camera,
            player.position,
            0.5,
            Color::WHITE,
        );
    }

    fn draw_impl(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::rgb(0.05, 0.05, 0.2)), None);
        self.draw_player(framebuffer, &self.player);
        for player in self.model.players.values() {
            if player.id != self.player.id {
                self.draw_player(framebuffer, player);
            }
        }
    }
    fn update_player(&mut self, delta_time: f32) {
        self.player.target_velocity = vec2(0.0, 0.0);
        if self.geng.window().is_key_pressed(geng::Key::A)
            || self.geng.window().is_key_pressed(geng::Key::Left)
        {
            self.player.target_velocity.x -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::D)
            || self.geng.window().is_key_pressed(geng::Key::Right)
        {
            self.player.target_velocity.x += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::W)
            || self.geng.window().is_key_pressed(geng::Key::Up)
        {
            self.player.target_velocity.y += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::S)
            || self.geng.window().is_key_pressed(geng::Key::Down)
        {
            self.player.target_velocity.y -= 1.0;
        }
        self.player.update(delta_time);
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.draw_impl(framebuffer);
    }
    fn update(&mut self, delta_time: f64) {
        let mut messages = Vec::new();
        match &mut self.connection {
            Connection::Remote(connection) => messages.extend(connection.new_messages()),
            Connection::Local { next_tick, model } => {
                *next_tick -= delta_time;
                while *next_tick <= 0.0 {
                    messages.push(ServerMessage::Update(model.tick()));
                    *next_tick += 1.0 / model.ticks_per_second;
                }
            }
        }
        let mut messages_to_send = mem::replace(&mut self.to_send, Vec::new());
        if !messages.is_empty() {
            messages_to_send.push(ClientMessage::Event(Event::PlayerUpdated(
                self.player.clone(),
            )));
        }
        for message in messages_to_send {
            match &mut self.connection {
                Connection::Remote(connection) => connection.send(message),
                Connection::Local {
                    next_tick: _,
                    model,
                } => {
                    messages.push(ServerMessage::Update(
                        model.handle_message(self.player.id, message),
                    ));
                }
            }
        }
        for message in messages {
            match message {
                ServerMessage::Update(events) => {
                    for event in events {
                        match event {
                            _ => {}
                        }
                        self.model.handle(event);
                    }
                }
                _ => unreachable!(),
            }
        }
        let delta_time = delta_time as f32;
        self.update_player(delta_time);
        for player in self.model.players.values_mut() {
            player.update(delta_time);
        }

        for player in self.model.players.values() {
            if player.id == self.player.id {
                continue;
            }
            self.players
                .entry(player.id)
                .or_default()
                .update(player, delta_time);
        }
        self.players
            .entry(self.player.id)
            .or_default()
            .update(&self.player, delta_time);
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            _ => {}
        }
    }
    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
    }
}
