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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Seat {
    pub position: Vec2<f32>,
    pub leave_position: Vec2<f32>,
    pub radius: f32,
    pub person: Option<Id>,
    pub order: Option<Order>,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
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
}

impl Boss {
    const WALK_SPEED: f32 = 4.0;
    const RUN_SPEED: f32 = 10.0;
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
    pub fn new() -> Self {
        let mut tables = Vec::new();
        let mut seats = Vec::new();
        for x in -2..=2 {
            for y in -2..0 {
                let table_pos = vec2(x as f32, y as f32) * 5.0;
                let table_radius = 1.0;
                tables.push(Table {
                    position: table_pos,
                    radius: table_radius,
                });
                const SEATS: usize = 6;
                for i in 0..SEATS {
                    seats.push(Seat {
                        position: table_pos
                            + Vec2::rotated(
                                vec2(table_radius + 0.1, 0.0),
                                2.0 * f32::PI * i as f32 / SEATS as f32,
                            ),
                        leave_position: table_pos
                            + Vec2::rotated(
                                vec2(table_radius + 0.1 + 0.4 + 0.5 + 0.15, 0.0),
                                2.0 * f32::PI * i as f32 / SEATS as f32,
                            ),
                        radius: 0.4,
                        person: None,
                        order: None,
                    });
                }
            }
        }
        let mut kitchen = vec![
            KitchenThing {
                typ: KitchenThingType::Dough,
                position: vec2(-2.0, 2.0),
                radius: 0.8,
            },
            KitchenThing {
                typ: KitchenThingType::TrashCan,
                position: vec2(7.0, 5.0),
                radius: 0.7,
            },
            KitchenThing {
                typ: KitchenThingType::Oven,
                position: vec2(7.0, 2.0),
                radius: 1.0,
            },
        ];
        {
            let mut x = 0.0;
            for ingredient in Ingredient::all() {
                kitchen.push(KitchenThing {
                    typ: KitchenThingType::IngredientBox(ingredient),
                    position: vec2(x, 5.0),
                    radius: 0.7,
                });
                x += 1.5;
            }
        }
        let mut pathfind_nodes = Vec::new();
        let mut pathfind_edges;
        {
            const STEP: f32 = 0.5;
            let mut x = -15.0;
            while x <= 15.0 {
                let mut y = -15.0;
                while y <= 7.0 {
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
        let boss_node = self.find_node(self.boss.position);
        let boss_target_node = self.find_node(match self.boss.target {
            BossTarget::Hire(id) | BossTarget::Fire(id) => self
                .players
                .get(&id)
                .map_or(self.boss.position, |player| player.position),
            BossTarget::Walk(pos) => pos,
        });
        if boss_node == boss_target_node {
            self.boss.target =
                BossTarget::Walk(*self.pathfind_nodes.choose(&mut global_rng()).unwrap());
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
    PlayerJoined(Player),
    PlayerUpdated(Player),
    PlayerLeft(Id),
    BossUpdate(Boss),
    Order(usize, Option<Order>),
}
