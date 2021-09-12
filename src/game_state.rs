use super::*;

struct PlayerState {}

impl PlayerState {
    pub fn new() -> Self {
        Self {}
    }
    pub fn update(&mut self, player: &Player, delta_time: f32) {}
}

#[derive(Copy, Clone)]
enum ButtonType {
    MakeOrder,
    ToggleIngredient(Ingredient),
}

impl ButtonType {
    fn color(self) -> Color<f32> {
        match self {
            Self::MakeOrder => Color::WHITE,
            Self::ToggleIngredient(ingredient) => ingredient.color(),
        }
    }
}

struct Button {
    position: Vec2<f32>,
    radius: f32,
    typ: ButtonType,
}

impl Model {
    fn buttons_for(&self, seat: &Seat) -> Vec<Button> {
        let table = self
            .tables
            .iter()
            .min_by_key(|table| r32((table.position - seat.position).len()))
            .unwrap();
        let mut buttons = vec![Button {
            position: table.position + vec2(0.0, -0.5),
            radius: 0.2,
            typ: ButtonType::MakeOrder,
        }];
        let mut positions = vec![
            vec2(-1.0, -1.0),
            vec2(-1.0, 1.0),
            vec2(1.0, -1.0),
            vec2(1.0, 1.0),
        ]
        .into_iter();
        for ingredient in Ingredient::all() {
            buttons.push(Button {
                position: table.position + positions.next().unwrap() * 0.25 + vec2(0.0, 0.2),
                radius: 0.2,
                typ: ButtonType::ToggleIngredient(ingredient),
            });
        }
        buttons
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameState {
    geng: Geng,
    current_order: BTreeSet<Ingredient>,
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
            current_order: BTreeSet::new(),
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
            match player.unemployed_time {
                Some(_) => Color::WHITE,
                None => Color::rgb(0.7, 0.7, 0.7),
            },
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
        self.draw_player(framebuffer, &self.player);
        for player in self.model.players.values() {
            if player.id != self.player.id {
                self.draw_player(framebuffer, player);
            }
        }
        for seat in &self.model.seats {
            self.geng.draw_2d().circle(
                framebuffer,
                &self.camera,
                seat.position,
                seat.radius,
                Color::GRAY,
            );
        }

        for table in &self.model.tables {
            self.geng.draw_2d().circle(
                framebuffer,
                &self.camera,
                table.position,
                table.radius,
                Color::rgb(0.6, 0.6, 0.6),
            );
        }

        for seat in &self.model.seats {
            if let Some(order) = &seat.order {
                self.draw_ingredients(framebuffer, order, seat.position);
            }
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
                    KitchenThingType::IngredientBox(ingredient) => ingredient.color(),
                },
            );
        }

        if let Some(seat_index) = self.player.seat {
            let seat = &self.model.seats[seat_index];
            if seat.order.is_none() {
                for button in self.model.buttons_for(seat) {
                    if let ButtonType::ToggleIngredient(ingredient) = button.typ {
                        if self.current_order.contains(&ingredient) {
                            self.geng.draw_2d().circle(
                                framebuffer,
                                &self.camera,
                                button.position,
                                button.radius + 0.05,
                                Color::BLACK,
                            );
                        }
                    }
                    self.geng.draw_2d().circle(
                        framebuffer,
                        &self.camera,
                        button.position,
                        button.radius,
                        button.typ.color(),
                    );
                }
            }
        }

        self.geng.draw_2d().circle(
            framebuffer,
            &self.camera,
            self.model.boss.position,
            self.model.boss.size,
            Color::MAGENTA,
        );

        // for &node in &self.model.pathfind_nodes {
        //     self.geng
        //         .draw_2d()
        //         .circle(framebuffer, &self.camera, node, 0.1, Color::GRAY);
        // }
    }
    fn update_player(&mut self, delta_time: f32) {
        if let Some(time) = &mut self.player.unemployed_time {
            *time += delta_time;
        }
        self.player.target_velocity = vec2(0.0, 0.0);
        match self.player.seat {
            Some(seat_index) => {
                self.player.velocity = vec2(0.0, 0.0);
                self.player.position = self.model.seats[seat_index].position;
            }
            None => {
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
                for (seat_index, seat) in self.model.seats.iter().enumerate() {
                    if self.player.collide(seat.position, seat.radius) {
                        match self.player.unemployed_time {
                            Some(_) => {
                                if !self
                                    .model
                                    .players
                                    .values()
                                    .any(|player| player.seat == Some(seat_index))
                                {
                                    self.player.seat = Some(seat_index);
                                    self.current_order = BTreeSet::new();
                                }
                            }
                            None => {
                                if let Some(order) = &seat.order {
                                    if let Some(pizza) = &self.player.pizza {
                                        if pizza.state == PizzaState::Cooked {
                                            if order == &pizza.ingredients {
                                                // TODO: add score
                                                self.player.pizza = None;
                                                self.to_send.push(ClientMessage::Event(
                                                    Event::Order(seat_index, None),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
                                        ingredients: BTreeSet::new(),
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
                            KitchenThingType::TrashCan => {
                                self.player.pizza = None;
                            }
                        }
                    }
                }
            }
        }
        if self.player.unemployed_time.is_some() {
            self.player.pizza = None;
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
            },
        );
        self.draw_ingredients(framebuffer, &pizza.ingredients, position + vec2(0.0, 0.3));
    }
    fn draw_ingredients(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        ingredients: &BTreeSet<Ingredient>,
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

    fn update_camera(&mut self, delta_time: f32) {
        let target_camera_position = self.player.position;
        let mut target_camera_fov = 20.0;
        if let Some(seat_index) = self.player.seat {
            if self.model.seats[seat_index].order.is_none() {
                target_camera_fov = 10.0;
            }
        }
        self.camera.center +=
            (target_camera_position - self.camera.center) * (delta_time * 5.0).min(1.0);
        self.camera.max_vertical_fov +=
            (target_camera_fov - self.camera.max_vertical_fov) * (delta_time * 5.0).min(1.0);
        self.camera.max_horizontal_fov = self.camera.max_vertical_fov;
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
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
        // for player in self.model.players.values_mut() {
        //     player.update(delta_time);
        // }
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

        self.update_camera(delta_time);
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown {
                button: geng::MouseButton::Left,
                position,
            } => {
                let position = position.map(|x| x as f32);
                let position = self.camera.screen_to_world(self.framebuffer_size, position);
                if let Some(seat_index) = self.player.seat {
                    let seat = &self.model.seats[seat_index];
                    if seat.order.is_none() {
                        for button in self.model.buttons_for(seat) {
                            if (position - button.position).len() < button.radius {
                                match button.typ {
                                    ButtonType::ToggleIngredient(ingredient) => {
                                        if !self.current_order.remove(&ingredient) {
                                            self.current_order.insert(ingredient);
                                        }
                                    }
                                    ButtonType::MakeOrder => {
                                        if !self.current_order.is_empty() {
                                            self.to_send.push(ClientMessage::Event(Event::Order(
                                                seat_index,
                                                Some(self.current_order.clone()),
                                            )));
                                            self.current_order.clear();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::W
                | geng::Key::A
                | geng::Key::S
                | geng::Key::D
                | geng::Key::Left
                | geng::Key::Right
                | geng::Key::Up
                | geng::Key::Down => {
                    if let Some(seat_index) = self.player.seat {
                        self.player.seat = None;
                        self.player.position = self.model.seats[seat_index].leave_position;
                        self.to_send
                            .push(ClientMessage::Event(Event::Order(seat_index, None)));
                    }
                }
                geng::Key::T => {
                    self.player.unemployed_time = match self.player.unemployed_time {
                        Some(_) => None,
                        None => Some(0.0),
                    };
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
    }
}
