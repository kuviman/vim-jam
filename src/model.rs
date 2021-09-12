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
pub struct Model {
    id_gen: IdGen,
    pub ticks_per_second: f64,
    pub players: HashMap<Id, Player>,
    pub tables: Vec<Table>,
    pub seats: Vec<Seat>,
    pub kitchen: Vec<KitchenThing>,
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
        let mut model = Self {
            id_gen: IdGen::new(),
            ticks_per_second: 20.0,
            players: default(),
            tables,
            seats,
            kitchen: {
                let mut things = vec![
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
                let mut x = 0.0;
                for ingredient in Ingredient::all() {
                    things.push(KitchenThing {
                        typ: KitchenThingType::IngredientBox(ingredient),
                        position: vec2(x, 5.0),
                        radius: 0.7,
                    });
                    x += 1.5;
                }
                things
            },
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
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    PlayerJoined(Player),
    PlayerUpdated(Player),
    PlayerLeft(Id),
    Order(usize, Option<Order>),
}
