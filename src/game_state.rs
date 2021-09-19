use std::collections::BTreeMap;

use super::*;

struct PlayerState {
    position: Vec2<f32>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            position: vec2(0.0, 0.0),
        }
    }
    pub fn update(&mut self, player: &Player, delta_time: f32) {
        self.position += (player.position - self.position) * (delta_time * 5.0).min(1.0);
    }
}

#[derive(Copy, Clone)]
enum ButtonType {
    MakeOrder,
    ToggleIngredient(Ingredient),
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
    last_interaction_time: HashMap<KitchenThingType, f32>,
    t: f32,
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
    boss_position: Vec2<f32>,
    boss_hop: f32,
    boss_left: bool,
}

impl Drop for GameState {
    fn drop(&mut self) {
        if let Connection::Remote(connection) = &mut self.connection {
            connection.send(ClientMessage::Event(Event::PlayerLeft(self.player.id)));
        }
    }
}

type RenderQ<'a> = BTreeMap<R32, Vec<Box<dyn Fn(&mut ugli::Framebuffer) + 'a>>>;

impl GameState {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        opt: &Rc<Opt>,
        name: &str,
        color: Color<f32>,
        welcome: WelcomeMessage,
        connection: Connection,
    ) -> Self {
        let mut player = welcome.model.players[&welcome.player_id].clone();
        player.name = name.to_owned();
        player.color = color;
        Self {
            boss_left: true,
            boss_hop: 0.0,
            boss_position: welcome.model.boss.position,
            t: 0.0,
            last_interaction_time: default(),
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

    fn draw_player<'a, 'b>(&'a self, renderq: &'b mut RenderQ<'a>, player: &'a Player) {
        let player_position = if player.id == self.player.id {
            player.position
        } else {
            self.players
                .get(&player.id)
                .map(|player| player.position)
                .unwrap_or(player.position)
        };
        let mut aabb = AABB::pos_size(
            player_position
                - vec2(
                    player.radius,
                    player.radius
                        + (player.t * 20.0).sin().abs()
                            * player.velocity.len().min(1.0)
                            * player.radius
                            * 0.2,
                ),
            vec2(1.0, 1.0) * player.radius * 2.0,
        );
        let mut left = player.left;
        if let Some(seat) = player.seat {
            let seat = &self.model.seats[seat];
            left = seat.leave_position.x > seat.position.x;
            aabb = aabb.translate(vec2(0.0, player.radius));
        }
        let initial_aabb = aabb;
        if left {
            mem::swap(&mut aabb.x_min, &mut aabb.x_max);
        }
        let layer = r32(if player.seat.is_some() {
            player_position.y - 0.3
        } else {
            player_position.y
        });
        let layer = renderq.entry(layer).or_default();
        layer.push(Box::new(move |framebuffer| {
            self.geng.draw_2d().textured_quad(
                framebuffer,
                &self.camera,
                aabb,
                if player.seat.is_some() {
                    &self.assets.monke_sit
                } else if player.pizza.is_some() {
                    &self.assets.monke_up
                } else {
                    &self.assets.monke_down
                },
                Color::WHITE,
            );
            self.geng.draw_2d().textured_quad(
                framebuffer,
                &self.camera,
                aabb,
                if player.seat.is_some() {
                    &self.assets.monke_sit_color
                } else if player.pizza.is_some() {
                    &self.assets.monke_up_color
                } else {
                    &self.assets.monke_down_color
                },
                player.color,
            );
            if player.unemployed_time.is_none() {
                self.geng.draw_2d().textured_quad(
                    framebuffer,
                    &self.camera,
                    initial_aabb,
                    if left {
                        &self.assets.badge_left
                    } else {
                        &self.assets.badge_right
                    },
                    Color::WHITE,
                );
            }
            // self.geng.draw_2d().circle(
            //     framebuffer,
            //     &self.camera,
            //     player.position,
            //     player.radius,
            //     match player.unemployed_time {
            //         Some(_) => Color::WHITE,
            //         None => Color::rgb(0.7, 0.7, 0.7),
            //     },
            // );
            if let Some(pizza) = &player.pizza {
                self.geng.draw_2d().textured_quad(
                    framebuffer,
                    &self.camera,
                    initial_aabb.translate(vec2(0.0, self.player.radius)),
                    match pizza.state {
                        PizzaState::Cooked => &self.assets.pizza,
                        PizzaState::Raw => &self.assets.raw_pizza,
                    },
                    Color::WHITE,
                );
            }
        }));
    }

    fn draw_impl(&self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::rgb(0.9, 0.9, 0.8)), None);

        let mut renderq = RenderQ::new();

        self.geng.draw_2d().textured(
            framebuffer,
            &self.camera,
            &[
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-1.25, -100.0),
                    a_pos: vec2(-1.25, -100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-1.25, 100.0),
                    a_pos: vec2(-1.25, 100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-100.0, 100.0),
                    a_pos: vec2(-100.0, 100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-100.0, -100.0),
                    a_pos: vec2(-100.0, -100.0),
                    a_color: Color::WHITE,
                },
            ],
            &self.assets.floor,
            Color::rgb(0.9, 0.9, 0.8),
            ugli::DrawMode::TriangleFan,
        );
        self.geng.draw_2d().textured(
            framebuffer,
            &self.camera,
            &[
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-1.25, -100.0),
                    a_pos: vec2(-1.25, -100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(-1.25, 100.0),
                    a_pos: vec2(-1.25, 100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(100.0, 100.0),
                    a_pos: vec2(100.0, 100.0),
                    a_color: Color::WHITE,
                },
                geng::draw_2d::TexturedVertex {
                    a_vt: vec2(100.0, -100.0),
                    a_pos: vec2(100.0, -100.0),
                    a_color: Color::WHITE,
                },
            ],
            &self.assets.floor,
            Color::rgb(0.9, 0.8, 0.8),
            ugli::DrawMode::TriangleFan,
        );

        self.draw_player(&mut renderq, &self.player);
        for player in self.model.players.values() {
            if player.id != self.player.id {
                self.draw_player(&mut renderq, player);
            }
        }

        for seat in &self.model.seats {
            self.geng.draw_2d().ellipse(
                framebuffer,
                &self.camera,
                seat.position - vec2(0.0, seat.radius),
                vec2(seat.radius, seat.radius * 0.5) * 0.7,
                Color::rgba(0.0, 0.0, 0.0, 0.3),
            );
            renderq
                .entry(r32(seat.position.y + 0.05))
                .or_default()
                .push(Box::new(move |framebuffer| {
                    self.geng.draw_2d().textured_quad(
                        framebuffer,
                        &self.camera,
                        AABB::pos_size(
                            seat.position - vec2(seat.radius, seat.radius * 1.3),
                            vec2(seat.radius, seat.radius) * 2.0,
                        ),
                        &self.assets.stool,
                        seat.color,
                    );
                }));
        }

        for table in &self.model.tables {
            self.geng.draw_2d().ellipse(
                framebuffer,
                &self.camera,
                table.position - vec2(0.0, table.radius * 0.2),
                vec2(table.radius, table.radius * 0.7),
                Color::rgba(0.0, 0.0, 0.0, 0.3),
            );
            renderq
                .entry(r32(table.position.y - table.radius * 0.5))
                .or_default()
                .push(Box::new(move |framebuffer| {
                    self.geng.draw_2d().textured_quad(
                        framebuffer,
                        &self.camera,
                        AABB::pos_size(
                            table.position - vec2(table.radius, table.radius * 0.9),
                            vec2(table.radius, table.radius) * 2.0,
                        ),
                        &self.assets.table,
                        table.color,
                    );
                }));
        }

        for thing in &self.model.kitchen {
            self.geng.draw_2d().ellipse(
                framebuffer,
                &self.camera,
                thing.position
                    - vec2(
                        0.0,
                        if thing.typ == KitchenThingType::Dough {
                            thing.radius * 0.4
                        } else {
                            thing.radius * 0.7
                        },
                    ),
                vec2(thing.radius, thing.radius * 0.5),
                Color::rgba(0.0, 0.0, 0.0, 0.3),
            );
            let interacted = self
                .last_interaction_time
                .get(&thing.typ)
                .copied()
                .unwrap_or(-100.0)
                > self.t - 0.5;
            match thing.typ {
                KitchenThingType::IngredientBox(ingredient) => {
                    self.geng.draw_2d().textured_quad(
                        framebuffer,
                        &self.camera,
                        AABB::pos_size(
                            thing.position - vec2(1.0, 1.0) * thing.radius,
                            vec2(1.0, 1.0) * thing.radius * 2.0,
                        ),
                        &self.assets.r#box,
                        Color::WHITE,
                    );
                    self.draw_ingredient(
                        framebuffer,
                        ingredient,
                        thing.position,
                        thing.radius * 0.5,
                    );
                }
                _ => {
                    self.geng.draw_2d().textured_quad(
                        framebuffer,
                        &self.camera,
                        AABB::pos_size(
                            thing.position - vec2(1.0, 1.0) * thing.radius,
                            vec2(1.0, 1.0) * thing.radius * 2.0,
                        ),
                        match thing.typ {
                            KitchenThingType::Oven => {
                                if interacted {
                                    &self.assets.oven_opened
                                } else {
                                    &self.assets.oven
                                }
                            }
                            KitchenThingType::TrashCan => {
                                if interacted {
                                    &self.assets.trash_opened
                                } else {
                                    &self.assets.trash
                                }
                            }
                            KitchenThingType::Dough => &self.assets.dough,
                            _ => unreachable!(),
                        },
                        Color::WHITE,
                    );
                }
            }
        }

        renderq
            .entry(r32(self.boss_position.y))
            .or_default()
            .push(Box::new(move |framebuffer| {
                let radius = 0.8;
                let mut aabb = AABB::pos_size(
                    self.boss_position - vec2(radius, radius * 0.9),
                    vec2(radius, radius) * 2.0,
                )
                .translate(vec2(
                    0.0,
                    (self.t * 15.0).sin().abs() * self.boss_hop.min(1.0) * 0.1,
                ));
                if !self.boss_left {
                    mem::swap(&mut aabb.x_min, &mut aabb.x_max);
                }
                self.geng.draw_2d().textured_quad(
                    framebuffer,
                    &self.camera,
                    aabb,
                    &self.assets.boss,
                    Color::WHITE,
                );
            }));
        for (_layer, rens) in renderq.into_iter().rev() {
            for ren in rens {
                ren(framebuffer);
            }
        }

        for player in self.model.players.values() {
            if player.name.is_empty() {
                continue;
            }
            if let Some(pos) = self.camera.world_to_screen(
                self.framebuffer_size,
                player.position
                    + vec2(
                        0.0,
                        if let Some(seat) = player.seat {
                            let seat = &self.model.seats[seat];
                            if seat.order.is_some() {
                                player.radius * 2.7
                            } else {
                                player.radius * 2.1
                            }
                        } else {
                            if player.pizza.is_some() {
                                player.radius * 2.3
                            } else {
                                player.radius * 1.5
                            }
                        },
                    ),
            ) {
                self.assets.font.draw_aligned(
                    framebuffer,
                    &geng::PixelPerfectCamera,
                    &player.name,
                    pos,
                    0.5,
                    20.0,
                    Color::rgba(0.0, 0.0, 0.0, 0.5),
                );
            }
        }

        for seat in &self.model.seats {
            if let Some(order) = &seat.order {
                self.draw_ingredients(framebuffer, order, seat.position + vec2(0.0, 1.0));
            }
        }
        for player in self.model.players.values() {
            if let Some(pizza) = &player.pizza {
                self.draw_pizza(
                    framebuffer,
                    pizza,
                    player.position + vec2(0.0, player.radius),
                );
            }
        }

        if let Some(seat_index) = self.player.seat {
            let seat = &self.model.seats[seat_index];
            if seat.order.is_none() {
                for button in self.model.buttons_for(seat) {
                    match button.typ {
                        ButtonType::MakeOrder => {
                            self.geng.draw_2d().textured_quad(
                                framebuffer,
                                &self.camera,
                                AABB::pos_size(
                                    button.position - vec2(button.radius, button.radius),
                                    vec2(1.0, 1.0) * button.radius * 2.0,
                                ),
                                &self.assets.order,
                                Color::WHITE,
                            );
                        }
                        ButtonType::ToggleIngredient(ingredient) => {
                            self.draw_ingredient(
                                framebuffer,
                                ingredient,
                                button.position,
                                button.radius,
                            );
                        }
                    }
                    if let ButtonType::ToggleIngredient(ingredient) = button.typ {
                        if !self.current_order.contains(&ingredient) {
                            self.geng.draw_2d().circle(
                                framebuffer,
                                &self.camera,
                                button.position,
                                button.radius,
                                Color::rgba(0.0, 0.0, 0.0, 0.7),
                            );
                        }
                    }
                }
            }
        }

        self.assets.font.draw(
            framebuffer,
            &geng::PixelPerfectCamera,
            &format!(
                "Next firing: {:.0}s",
                (Boss::FIRE_TIMER - self.model.boss.timer).max(0.0)
            ),
            vec2(10.0, 10.0),
            48.0,
            Color::BLACK,
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
                                                self.player.pizza = None;
                                                self.player.score += 1;
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
                                    self.to_send
                                        .push(ClientMessage::Event(Event::Interacted(thing.typ)));
                                }
                            }
                            KitchenThingType::IngredientBox(ingredient) => {
                                if let Some(pizza) = &mut self.player.pizza {
                                    if pizza.state == PizzaState::Raw {
                                        if pizza.ingredients.insert(ingredient) {
                                            self.to_send.push(ClientMessage::Event(
                                                Event::Interacted(thing.typ),
                                            ));
                                        }
                                    }
                                }
                            }
                            KitchenThingType::Oven => {
                                if let Some(pizza) = &mut self.player.pizza {
                                    if pizza.state == PizzaState::Raw {
                                        pizza.state = PizzaState::Cooked;
                                        self.to_send.push(ClientMessage::Event(Event::Interacted(
                                            thing.typ,
                                        )));
                                    }
                                }
                            }
                            KitchenThingType::TrashCan => {
                                if self.player.pizza.is_some() {
                                    self.to_send
                                        .push(ClientMessage::Event(Event::Interacted(thing.typ)));
                                    self.player.pizza = None;
                                }
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
        // self.geng.draw_2d().circle(
        //     framebuffer,
        //     &self.camera,
        //     position,
        //     0.3,
        //     match pizza.state {
        //         PizzaState::Raw => Color::rgb(1.0, 1.0, 0.7),
        //         PizzaState::Cooked => Color::rgb(0.7, 0.7, 0.4),
        //     },
        // );
        self.draw_ingredients(framebuffer, &pizza.ingredients, position + vec2(0.0, 0.3));
    }
    fn draw_ingredients(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        ingredients: &BTreeSet<Ingredient>,
        position: Vec2<f32>,
    ) {
        for (index, &ingredient) in ingredients.iter().enumerate() {
            self.draw_ingredient(
                framebuffer,
                ingredient,
                position + vec2(0.4 * index as f32, 0.0),
                0.2,
            );
        }
    }

    fn update_camera(&mut self, delta_time: f32) {
        let mut camera = self.camera.clone();
        camera.center = self.player.position;
        let top_right = camera.screen_to_world(self.framebuffer_size, self.framebuffer_size);
        if top_right.x > 5.0 {
            camera.center.x -= top_right.x - 5.0;
        }
        if top_right.y > 5.0 {
            camera.center.y -= top_right.y - 5.0;
        }
        let bottom_left = camera.screen_to_world(self.framebuffer_size, vec2(0.0, 0.0));
        if bottom_left.x < -15.0 {
            camera.center.x += -15.0 - bottom_left.x;
        }
        if bottom_left.y < -5.0 {
            camera.center.y += -5.0 - bottom_left.y;
        }
        let mut target_camera_position = camera.center;
        let mut target_camera_fov = 20.0;
        if let Some(seat_index) = self.player.seat {
            if self.model.seats[seat_index].order.is_none() {
                target_camera_fov = 10.0;
            }
        }
        self.camera.center +=
            (target_camera_position - self.camera.center) * (delta_time * 5.0).min(1.0);
        self.camera.max_horizontal_fov +=
            (target_camera_fov - self.camera.max_horizontal_fov) * (delta_time * 5.0).min(1.0);
        self.camera.max_vertical_fov = self.camera.max_horizontal_fov.min(10.0);
    }

    pub(crate) fn draw_ingredient(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        ingredient: Ingredient,
        position: Vec2<f32>,
        radius: f32,
    ) {
        self.geng.draw_2d().circle(
            framebuffer,
            &self.camera,
            position,
            radius + 0.03,
            Color::BLACK,
        );
        self.geng.draw_2d().circle(
            framebuffer,
            &self.camera,
            position,
            radius,
            ingredient.color(),
        );
        self.geng.draw_2d().textured_quad(
            framebuffer,
            &self.camera,
            AABB::pos_size(
                position - vec2(radius, radius) * 0.8,
                vec2(radius, radius) * 2.0 * 0.8,
            ),
            self.assets.texture_for(ingredient),
            Color::WHITE,
        );
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.draw_impl(framebuffer);
    }
    fn update(&mut self, delta_time: f64) {
        self.t += delta_time as f32;
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
                            Event::Hire(id) if id == self.player.id => {
                                self.player.unemployed_time = None;
                                if let Some(seat_index) = self.player.seat {
                                    self.player.seat = None;
                                    self.player.position =
                                        self.model.seats[seat_index].leave_position;
                                    self.to_send
                                        .push(ClientMessage::Event(Event::Order(seat_index, None)));
                                }
                            }
                            Event::Fire(id) if id == self.player.id => {
                                self.player.unemployed_time = Some(0.0);
                            }
                            Event::Interacted(typ) => {
                                self.last_interaction_time.insert(typ, self.t);
                            }
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

        self.update_camera(delta_time);
        self.model
            .players
            .insert(self.player.id, self.player.clone());

        let delta_boss_position =
            (self.model.boss.position - self.boss_position) * (delta_time * 5.0).min(1.0);
        self.boss_position += delta_boss_position;
        let boss_velocity = delta_boss_position / delta_time;
        self.boss_hop = boss_velocity.len();
        self.boss_left = boss_velocity.x < 0.0;
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
                    if self.player.unemployed_time.is_none() {
                        self.player.unemployed_time = Some(0.0);
                    } else {
                        self.player.unemployed_time = None;
                    }
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
