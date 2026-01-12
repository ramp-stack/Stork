use prism::event::OnEvent;
use prism::drawable::{Drawable, Component};
use prism::Context;
use prism::layout::{Area, SizeRequest};
use prism::canvas::Image;

use crate::animation::AnimatedSprite;

#[derive(Debug, Clone)]
pub enum Target {
    ByName(String),
    ById(String),
    ByTag(String),
}

#[derive(Debug, Clone)]
pub enum Location {
    Position((f32, f32)),
    Between(Box<Target>, Box<Target>),
    AtTarget(Box<Target>),
}

#[derive(Debug, Clone)]
pub enum Action {
    ApplyMomentum {
        target: Target,
        value: (f32, f32),
    },
    SetMomentum {
        target: Target,
        value: (f32, f32),
    },
    Spawn {
        object: Box<GameObject>,
        location: Location,
    },
    SetResistance {
        target: Target,
        value: (f32, f32),
    },
    Remove {
        target: Target,
    },
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    Collision {
        action: Action,
        target: Target,
    },
    BoundaryCollision {
        action: Action,
        target: Target,
    },
    KeyPress {
        key: prism::event::Key,
        action: Action,
        target: Target,
    },
}

#[derive(Clone, Debug)]
pub struct GameObject {
    pub id: String,
    pub tags: Vec<String>,
    image: Image,
    pub animated_sprite: Option<AnimatedSprite>,
    pub size: (f32, f32),
    pub position: (f32, f32),
    pub momentum: (f32, f32),
    pub resistance: (f32, f32),
    pub gravity: f32,
}

impl OnEvent for GameObject {}

impl Component for GameObject {
    fn children(&self) -> Vec<&dyn Drawable> {
        vec![&self.image]
    }
    
    fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
        vec![&mut self.image]
    }
    
    fn request_size(&self, _children: Vec<SizeRequest>) -> SizeRequest {
        SizeRequest::new(self.size.0, self.size.1, self.size.0, self.size.1)
    }
    
    fn build(&self, _size: (f32, f32), _children: Vec<SizeRequest>) -> Vec<Area> {
        vec![Area {
            offset: (0.0, 0.0),
            size: self.size
        }]
    }
}

impl GameObject {
    pub fn new(
        _ctx: &mut Context, 
        id: String, 
        image: Image, 
        size: f32, 
        position: (f32, f32),
        tags: Vec<String>,
        momentum: (f32, f32),
        resistance: (f32, f32),
        gravity: f32,
    ) -> Self {
        Self {
            id,
            tags,
            image,
            animated_sprite: None,
            size: (size, size),
            position,
            momentum,
            resistance,
            gravity,
        }
    }
    
    pub fn new_rect(
        _ctx: &mut Context, 
        id: String, 
        image: Image, 
        size: (f32, f32),  
        position: (f32, f32),
        tags: Vec<String>,
        momentum: (f32, f32),
        resistance: (f32, f32),
        gravity: f32,
    ) -> Self {
        Self {
            id,
            tags,
            image,
            animated_sprite: None,
            size,
            position,
            momentum,
            resistance,
            gravity,
        }
    }
    
    pub fn with_animation(mut self, animated_sprite: AnimatedSprite) -> Self {
        self.animated_sprite = Some(animated_sprite);
        self
    }
    
    pub fn set_gravity(&mut self, gravity: f32) {
        self.gravity = gravity;
    }
    
    pub fn update_position(&mut self) {
        self.position.0 += self.momentum.0;
        self.position.1 += self.momentum.1;
    }
    
    pub fn apply_gravity(&mut self) {
        self.momentum.1 += self.gravity;
    }
    
    pub fn apply_resistance(&mut self) {
        self.momentum.0 *= self.resistance.0;
        self.momentum.1 *= self.resistance.1;
        if self.momentum.0.abs() < 0.001 {
            self.momentum.0 = 0.0;
        }
        if self.momentum.1.abs() < 0.001 {
            self.momentum.1 = 0.0;
        }
    }
    
    pub fn update_animation(&mut self, delta_time: f32) {
        if let Some(sprite) = &mut self.animated_sprite {
            sprite.update(delta_time);
            self.image = sprite.get_current_image();
        }
    }
    
    pub fn check_boundary_collision(&self, canvas_size: (f32, f32)) -> bool {
        self.position.0 <= 0.0 ||
        self.position.0 + self.size.0 >= canvas_size.0 ||
        self.position.1 <= 0.0 ||
        self.position.1 + self.size.1 >= canvas_size.1
    }
}