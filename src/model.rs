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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: Id,
    pub position: Vec2<f32>,
    pub velocity: Vec2<f32>,
    pub target_velocity: Vec2<f32>,
}

impl Player {
    pub const SPEED: f32 = 4.0;
    pub const ACCELERATION: f32 = 30.0;
    pub fn new(id_gen: &mut IdGen) -> Self {
        let mut player = Self {
            id: id_gen.gen(),
            position: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            target_velocity: vec2(0.0, 0.0),
        };
        player
    }
    pub fn update(&mut self, delta_time: f32) {
        self.velocity += (self.target_velocity * Self::SPEED - self.velocity)
            .clamp(Self::ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    id_gen: IdGen,
    pub ticks_per_second: f64,
    pub players: HashMap<Id, Player>,
}

impl Model {
    pub fn new() -> Self {
        let mut model = Self {
            id_gen: IdGen::new(),
            ticks_per_second: 20.0,
            players: default(),
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
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    PlayerJoined(Player),
    PlayerUpdated(Player),
    PlayerLeft(Id),
}
