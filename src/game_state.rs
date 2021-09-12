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
            player.radius,
            Color::WHITE,
        );
        if let Some(pizza) = &player.pizza {
            self.draw_pizza(
                framebuffer,
                pizza,
                player.position + vec2(0.0, player.radius),
            );
        }
    }

    fn draw_impl(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::rgb(0.05, 0.05, 0.2)), None);
        self.camera.center = self.player.position;
        self.draw_player(framebuffer, &self.player);
        for player in self.model.players.values() {
            if player.id != self.player.id {
                self.draw_player(framebuffer, player);
            }
        }
        for table in &self.model.tables {
            self.geng.draw_2d().circle(
                framebuffer,
                &self.camera,
                table.position,
                table.radius,
                Color::GRAY,
            );
        }
        for thing in &self.model.kitchen {
            self.geng.draw_2d().circle(
                framebuffer,
                &self.camera,
                thing.position,
                thing.radius,
                match thing.typ {
                    KitchenThingType::Oven => Color::RED,
                    KitchenThingType::Dough => Color::rgb(1.0, 1.0, 0.5),
                    KitchenThingType::TrashCan => Color::GRAY,
                    KitchenThingType::Plates => Color::rgb(0.8, 0.8, 0.8),
                    KitchenThingType::IngredientBox(ingredient) => ingredient.color(),
                },
            );
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
        if self.player.target_velocity.len() > 0.1 {
            self.player.target_velocity = self.player.target_velocity.normalize();
        }
        self.player.update(delta_time);
        for table in &self.model.tables {
            self.player.collide(table.position, table.radius);
        }
        // for other_player in self.model.players.values() {
        //     if other_player.id == self.player.id {
        //         continue;
        //     }
        //     self.player
        //         .collide(other_player.position, other_player.radius);
        // }
        for thing in &self.model.kitchen {
            if self.player.collide(thing.position, thing.radius) {
                match thing.typ {
                    KitchenThingType::Dough => {
                        if self.player.pizza.is_none() {
                            self.player.pizza = Some(Pizza {
                                ingredients: HashSet::new(),
                                state: PizzaState::Raw,
                            });
                        }
                    }
                    KitchenThingType::IngredientBox(ingredient) => {
                        if let Some(pizza) = &mut self.player.pizza {
                            if pizza.state == PizzaState::Raw {
                                pizza.ingredients.insert(ingredient);
                            }
                        }
                    }
                    KitchenThingType::Oven => {
                        if let Some(pizza) = &mut self.player.pizza {
                            if pizza.state == PizzaState::Raw {
                                pizza.state = PizzaState::Cooked;
                            }
                        }
                    }
                    KitchenThingType::Plates => {
                        if let Some(pizza) = &mut self.player.pizza {
                            if pizza.state == PizzaState::Cooked {
                                pizza.state = PizzaState::Plated;
                            }
                        }
                    }
                    KitchenThingType::TrashCan => {
                        self.player.pizza = None;
                    }
                }
            }
        }
    }

    fn draw_pizza(&self, framebuffer: &mut ugli::Framebuffer, pizza: &Pizza, position: Vec2<f32>) {
        self.geng.draw_2d().circle(
            framebuffer,
            &self.camera,
            position,
            0.3,
            match pizza.state {
                PizzaState::Raw => Color::rgb(1.0, 1.0, 0.7),
                PizzaState::Cooked => Color::rgb(0.7, 0.7, 0.4),
                PizzaState::Plated => Color::rgb(0.5, 0.5, 0.4),
            },
        );
        self.draw_ingredients(framebuffer, &pizza.ingredients, position + vec2(0.0, 0.3));
    }
    fn draw_ingredients(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        ingredients: &HashSet<Ingredient>,
        position: Vec2<f32>,
    ) {
        for (index, &ingredient) in ingredients.iter().enumerate() {
            self.geng.draw_2d().circle(
                framebuffer,
                &self.camera,
                position + vec2(0.2 * index as f32, 0.0),
                0.1,
                ingredient.color(),
            );
        }
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
        for player in self.model.players.values_mut() {
            player.update(delta_time);
        }
        self.update_player(delta_time);

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
