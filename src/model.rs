use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Copy)]
pub struct Id(usize);

impl Id {
    pub fn raw(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdGen {
    next_id: usize,
}

impl IdGen {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }
    pub fn gen(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Copy)]
pub enum PizzaState {
    Raw,
    Cooked,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pizza {
    pub ingredients: BTreeSet<Ingredient>,
    pub state: PizzaState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: Id,
    pub score: i32,
    pub radius: f32,
    pub position: Vec2<f32>,
    pub velocity: Vec2<f32>,
    pub target_velocity: Vec2<f32>,
    pub pizza: Option<Pizza>,
    pub unemployed_time: Option<f32>,
    pub seat: Option<usize>,
}

impl Player {
    pub const SPEED: f32 = 6.0;
    pub const ACCELERATION: f32 = 50.0;
    pub fn new(id_gen: &mut IdGen) -> Self {
        let mut player = Self {
            id: id_gen.gen(),
            score: 0,
            radius: 0.5,
            position: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            target_velocity: vec2(0.0, 0.0),
            pizza: None,
            unemployed_time: Some(0.0),
            seat: None,
        };
        player
    }
    pub fn update(&mut self, delta_time: f32) {
        self.velocity += (self.target_velocity * Self::SPEED - self.velocity)
            .clamp(Self::ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
        self.position.x = clamp(self.position.x, -14.0..=4.0);
        self.position.y = clamp(self.position.y, -4.0..=4.0);
    }

    pub fn collide(&mut self, position: Vec2<f32>, radius: f32) -> bool {
        let distance = (self.position - position).len();
        if distance > 0.0001 && distance < radius + self.radius {
            self.position +=
                (self.position - position).normalize() * (radius + self.radius - distance);
            true
        } else {
            false
        }
    }
}

pub type Order = BTreeSet<Ingredient>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Table {
    pub position: Vec2<f32>,
    pub radius: f32,
    pub color: Color<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Seat {
    pub position: Vec2<f32>,
    pub leave_position: Vec2<f32>,
    pub radius: f32,
    pub order: Option<Order>,
    pub color: Color<f32>,
}

#[derive(Ord, PartialOrd, Debug, Serialize, Deserialize, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Ingredient {
    Cheese,
    Tomato,
    Cucumber,
    Pepperoni,
}

impl Ingredient {
    pub fn all() -> Vec<Self> {
        vec![Self::Cheese, Self::Tomato, Self::Cucumber, Self::Pepperoni]
    }
    pub fn color(self) -> Color<f32> {
        match self {
            Self::Cheese => Color::YELLOW,
            Self::Tomato => Color::RED,
            Self::Cucumber => Color::GREEN,
            Self::Pepperoni => Color::rgb(1.0, 0.5, 0.0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum KitchenThingType {
    Oven,
    Dough,
    TrashCan,
    IngredientBox(Ingredient),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KitchenThing {
    pub position: Vec2<f32>,
    pub radius: f32,
    pub typ: KitchenThingType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BossTarget {
    Walk(Vec2<f32>),
    Fire(Id),
    Hire(Id),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Boss {
    pub position: Vec2<f32>,
    pub size: f32,
    pub target: BossTarget,
    pub timer: f32,
}

impl Boss {
    pub const WALK_SPEED: f32 = 4.0;
    pub const RUN_SPEED: f32 = 10.0;
    pub const FIRE_TIMER: f32 = 30.0;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    id_gen: IdGen,
    pub boss: Boss,
    pub ticks_per_second: f64,
    pub players: HashMap<Id, Player>,
    pub tables: Vec<Table>,
    pub seats: Vec<Seat>,
    pub kitchen: Vec<KitchenThing>,
    pub pathfind_nodes: Vec<Vec2<f32>>,
    pub pathfind_edges: Vec<Vec<usize>>,
}

impl Model {
    pub const MAX_EMPLOYEES: usize = 5;
    pub fn new() -> Self {
        let mut tables = Vec::new();
        let mut seats = Vec::new();
        for x in -3..0 {
            for y in std::array::IntoIter::new([-1, 1]) {
                let table_pos = vec2(x as f32 * 4.0, y as f32 * 2.0);
                let table_radius = 1.0;
                tables.push(Table {
                    position: table_pos,
                    radius: table_radius,
                    color: hsv(global_rng().gen_range(0.0..=1.0), 0.2, 1.0),
                });
                const SEATS: usize = 6;
                for i in 0..SEATS {
                    seats.push(Seat {
                        position: table_pos + {
                            let mut pos = Vec2::rotated(
                                vec2(table_radius, 0.0),
                                2.0 * f32::PI * i as f32 / SEATS as f32 + f32::PI / 2.0,
                            );
                            pos.x *= 1.1;
                            pos.y *= 0.8;
                            pos
                        },
                        leave_position: table_pos
                            + Vec2::rotated(
                                vec2(table_radius + 0.1 + 0.4 + 0.5 + 0.15, 0.0),
                                2.0 * f32::PI * i as f32 / SEATS as f32 + f32::PI / 2.0,
                            ),
                        radius: 0.3,
                        order: None,
                        color: hsv(global_rng().gen_range(0.0..=1.0), 0.2, 0.8),
                    });
                }
            }
        }
        let mut kitchen = vec![
            KitchenThing {
                typ: KitchenThingType::Dough,
                position: vec2(1.0, -4.0),
                radius: 0.8,
            },
            KitchenThing {
                typ: KitchenThingType::TrashCan,
                position: vec2(3.5, 4.0),
                radius: 0.7,
            },
            KitchenThing {
                typ: KitchenThingType::Oven,
                position: vec2(1.0, 4.0),
                radius: 1.0,
            },
        ];
        {
            let mut x = -3.0;
            for ingredient in Ingredient::all() {
                kitchen.push(KitchenThing {
                    typ: KitchenThingType::IngredientBox(ingredient),
                    position: vec2(4.0, x),
                    radius: 0.7,
                });
                x += 1.5;
            }
        }
        let mut pathfind_nodes = Vec::new();
        let mut pathfind_edges;
        {
            const STEP: f32 = 0.5;
            let mut x = -14.0;
            while x <= 4.0 {
                let mut y = -4.0;
                while y <= 4.0 {
                    let pos = vec2(x, y);
                    let mut good = true;
                    for thing in &kitchen {
                        if (pos - thing.position).len() < thing.radius + 0.5 {
                            good = false;
                        }
                    }
                    for seat in &seats {
                        if (pos - seat.position).len() < seat.radius + 0.5 {
                            good = false;
                        }
                    }
                    for table in &tables {
                        if (pos - table.position).len() < table.radius + 0.5 {
                            good = false;
                        }
                    }
                    if good {
                        pathfind_nodes.push(pos);
                    }
                    y += STEP;
                }
                x += STEP;
            }
            pathfind_edges = vec![vec![]; pathfind_nodes.len()];
            for i in 0..pathfind_nodes.len() {
                for j in 0..pathfind_nodes.len() {
                    if (pathfind_nodes[i] - pathfind_nodes[j]).len() < STEP * 1.5 {
                        pathfind_edges[i].push(j);
                    }
                }
            }
        }
        let boss_pos = *pathfind_nodes.choose(&mut global_rng()).unwrap();
        let boss = Boss {
            timer: 0.0,
            position: boss_pos,
            size: 0.5,
            target: BossTarget::Walk(boss_pos),
        };
        let mut model = Self {
            id_gen: IdGen::new(),
            boss,
            ticks_per_second: 20.0,
            players: default(),
            tables,
            seats,
            kitchen,
            pathfind_nodes,
            pathfind_edges,
        };
        model
    }
    #[must_use]
    fn spawn_player(&mut self) -> (Id, Vec<Event>) {
        let player = Player::new(&mut self.id_gen);
        let events = vec![Event::PlayerJoined(player.clone())];
        let player_id = player.id;
        self.players.insert(player_id, player);
        (player_id, events)
    }
    #[must_use]
    pub fn welcome(&mut self) -> (WelcomeMessage, Vec<Event>) {
        let (player_id, events) = self.spawn_player();
        (
            WelcomeMessage {
                player_id,
                model: self.clone(),
            },
            events,
        )
    }
    #[must_use]
    pub fn drop_player(&mut self, player_id: Id) -> Vec<Event> {
        self.players.remove(&player_id);
        vec![Event::PlayerLeft(player_id)]
    }
    #[must_use]
    pub fn handle_message(
        &mut self,
        player_id: Id,
        message: ClientMessage,
        // sender: &mut dyn geng::net::Sender<ServerMessage>,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        match message {
            ClientMessage::Event(event) => {
                self.handle_impl(event.clone(), Some(&mut events));
                events.push(event);
            }
        }
        events
    }
    #[must_use]
    pub fn tick(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if self.players.is_empty() {
            return events;
        }
        let boss_node = self.find_node(self.boss.position);
        let boss_target_node = self.find_node(match self.boss.target {
            BossTarget::Hire(id) | BossTarget::Fire(id) => self
                .players
                .get(&id)
                .map_or(self.boss.position, |player| player.position),
            BossTarget::Walk(pos) => pos,
        });
        self.boss.timer += 1.0 / self.ticks_per_second as f32;
        if boss_node == boss_target_node {
            match self.boss.target {
                BossTarget::Fire(id) => {
                    self.boss.timer = 0.0;
                    events.push(Event::Reset);
                    events.push(Event::Fire(id));
                    self.handle(Event::Fire(id));
                    self.boss.target = BossTarget::Walk(self.boss.position);
                }
                BossTarget::Hire(id) => {
                    self.boss.timer = 0.0;
                    events.push(Event::Reset);
                    events.push(Event::Hire(id));
                    self.handle(Event::Hire(id));
                    self.boss.target = BossTarget::Walk(self.boss.position);
                }
                BossTarget::Walk(_) => {
                    let employees: Vec<&Player> = self
                        .players
                        .values()
                        .filter(|player| player.unemployed_time.is_none())
                        .collect();
                    let max_employees = Model::MAX_EMPLOYEES.min(self.players.len() / 3).max(1);
                    if employees.len() < max_employees {
                        self.boss.target = BossTarget::Hire(
                            self.players
                                .values()
                                .filter(|player| player.unemployed_time.is_some())
                                .max_by_key(|player| r32(player.unemployed_time.unwrap()))
                                .unwrap()
                                .id,
                        );
                    } else if self.boss.timer > Boss::FIRE_TIMER
                        && !employees.is_empty()
                        && self.players.len() >= 2
                    {
                        self.boss.target = BossTarget::Fire(
                            employees
                                .into_iter()
                                .min_by_key(|player| player.score)
                                .unwrap()
                                .id,
                        );
                    } else {
                        self.boss.target = BossTarget::Walk(
                            *self.pathfind_nodes.choose(&mut global_rng()).unwrap(),
                        );
                    }
                }
            }
        } else {
            let mut used = vec![false; self.pathfind_nodes.len()];
            let mut q = std::collections::BinaryHeap::new();
            let mut d = vec![f32::MAX; self.pathfind_nodes.len()];
            let mut p = vec![0; self.pathfind_nodes.len()];
            d[boss_target_node] = 0.0;
            q.push((r32(0.0), boss_target_node));
            while let Some((_, v)) = q.pop() {
                if used[v] {
                    continue;
                }
                if v == boss_node {
                    break;
                }
                used[v] = true;
                for u in self.pathfind_edges[v].iter().copied() {
                    let new_d = d[v] + (self.pathfind_nodes[v] - self.pathfind_nodes[u]).len();
                    if new_d < d[u] {
                        d[u] = new_d;
                        p[u] = v;
                        q.push((r32(-new_d), u));
                    }
                }
            }
            let next_node = p[boss_node];
            self.boss.position += (self.pathfind_nodes[next_node] - self.boss.position).clamp(
                match self.boss.target {
                    BossTarget::Walk(_) => Boss::WALK_SPEED,
                    _ => Boss::RUN_SPEED,
                } * 1.0
                    / self.ticks_per_second as f32,
            );
        }
        events.push(Event::BossUpdate(self.boss.clone()));
        events
    }
    pub fn handle(&mut self, event: Event) {
        self.handle_impl(event, None);
    }
    pub fn handle_impl(&mut self, event: Event, events: Option<&mut Vec<Event>>) {
        match event {
            Event::PlayerJoined(player) | Event::PlayerUpdated(player) => {
                let player_id = player.id;
                self.players.insert(player_id, player.clone());
            }
            Event::PlayerLeft(player_id) => {
                self.players.remove(&player_id);
            }
            Event::Order(seat_index, order) => {
                self.seats[seat_index].order = order;
            }
            Event::BossUpdate(boss) => {
                self.boss = boss;
            }
            Event::Fire(id) => {
                if let Some(player) = self.players.get_mut(&id) {
                    player.unemployed_time = Some(0.0);
                }
            }
            Event::Hire(id) => {
                if let Some(player) = self.players.get_mut(&id) {
                    player.unemployed_time = None;
                }
            }
            _ => {}
        }
    }

    fn find_node(&self, position: Vec2<f32>) -> usize {
        self.pathfind_nodes
            .iter()
            .enumerate()
            .min_by_key(|(_, &pos)| r32((pos - position).len()))
            .unwrap()
            .0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    Fire(Id),
    Hire(Id),
    Reset,
    PlayerJoined(Player),
    PlayerUpdated(Player),
    PlayerLeft(Id),
    BossUpdate(Boss),
    Order(usize, Option<Order>),
}
