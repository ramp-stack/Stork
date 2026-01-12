use prism::event::{OnEvent, Event, TickEvent, KeyboardEvent, KeyboardState};
use prism::drawable::Component;
use prism::Context;
use prism::layout::{Area, SizeRequest, Layout};
use std::collections::HashMap;
use std::cell::Cell;
use prism::drawable::SizedTree;

mod game_object;
mod animation;

pub use game_object::{GameObject, Action, Target, Location, GameEvent};
pub use animation::AnimatedSprite;

#[derive(Debug)]
pub struct CanvasLayout {
    offsets: Vec<(f32, f32)>,
    canvas_size: Cell<(f32, f32)>,
}

impl Layout for CanvasLayout {
    fn request_size(&self, _children: Vec<SizeRequest>) -> SizeRequest {
        SizeRequest::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn build(&self, size: (f32, f32), children: Vec<SizeRequest>) -> Vec<Area> {
        //TODO: 1. create safe areas to enforce 16:9 ratio
        //TODO: 2. scale all the children so that it fits inside 16:9 area with a size of 3840×2160

        self.canvas_size.set(size);
        
        if self.offsets.len() != children.len() {
            panic!("CanvasLayout does not have the same number of offsets as children!");
        }
        self.offsets.iter().copied().zip(children).map(|(offset, child)|
            Area {
                offset,
                size: child.get((f32::MAX, f32::MAX))  
            }
        ).collect()
    }
}

#[derive(Debug, Component)]
pub struct Canvas {
    layout: CanvasLayout,
    objects: Vec<GameObject>,
    #[skip] object_names: Vec<String>,
    #[skip] name_to_index: HashMap<String, usize>,
    #[skip] id_to_index: HashMap<String, usize>,
    #[skip] object_events: Vec<Vec<GameEvent>>,
    #[skip] tag_to_indices: HashMap<String, Vec<usize>>,
}

impl OnEvent for Canvas {
    fn on_event(&mut self, ctx: &mut Context, _tree: &SizedTree, event: Box<dyn Event>) -> Vec<Box<dyn Event>> {
        if let Some(KeyboardEvent { state: KeyboardState::Pressed, key }) = event.downcast_ref() {
            for idx in 0..self.objects.len() {
                if let Some(events) = self.object_events.get(idx).cloned() {
                    for game_event in events {
                        if let GameEvent::KeyPress { key: event_key, action, target: _ } = game_event {
                            if &event_key == key {
                                self.run(action);
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(_tick) = event.downcast_ref::<TickEvent>() {
            const DELTA_TIME: f32 = 0.016; 
            
            for idx in 0..self.objects.len() {
                if let Some(game_obj) = self.objects.get_mut(idx) {
                    game_obj.update_animation(DELTA_TIME);
                    game_obj.apply_gravity();
                    game_obj.update_position();
                    game_obj.apply_resistance();
                    self.layout.offsets[idx] = game_obj.position;
                }
            }
            
            for i in 0..self.objects.len() {
                for j in (i + 1)..self.objects.len() {
                    if self.check_collision(i, j) {
                        self.trigger_collision_events(i);
                        self.trigger_collision_events(j);
                    }
                }
            }
            
            let canvas_size = self.layout.canvas_size.get();
            let mut boundary_collisions = Vec::new();
            for idx in 0..self.objects.len() {
                if let Some(obj) = self.objects.get(idx) {
                    if obj.check_boundary_collision(canvas_size) {
                        boundary_collisions.push(idx);
                    }
                }
            }
            
            for idx in boundary_collisions {
                self.trigger_boundary_collision_events(idx);
            }
        }

        vec![event]
    }
}

impl Canvas {
    pub fn new(_ctx: &mut Context, _size: (f32, f32)) -> Self {
        Self {
            layout: CanvasLayout {
                offsets: Vec::new(),
                canvas_size: Cell::new((0.0, 0.0)),
            },
            objects: Vec::new(),
            object_names: Vec::new(),
            name_to_index: HashMap::new(),
            id_to_index: HashMap::new(),
            object_events: Vec::new(),
            tag_to_indices: HashMap::new(),
        }
    }
    
    pub fn update_size(&self, new_size: (f32, f32)) {
        self.layout.canvas_size.set(new_size);
    }
    
    pub fn get_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }
    
    pub fn add_game_object(&mut self, name: String, game_obj: GameObject) {
        let position = game_obj.position;
        let id = game_obj.id.clone();
        let tags = game_obj.tags.clone();
        
        let idx = self.objects.len();
        
        self.layout.offsets.push(position);
        self.name_to_index.insert(name.clone(), idx);
        self.id_to_index.insert(id.clone(), idx);
        
        for tag in tags {
            self.tag_to_indices.entry(tag).or_insert_with(Vec::new).push(idx);
        }
        
        self.object_names.push(name);
        self.objects.push(game_obj);
        self.object_events.push(Vec::new());
    }
    
    pub fn remove_game_object(&mut self, name: &str) {
        if let Some(&idx) = self.name_to_index.get(name) {
            let removed_name = self.object_names.remove(idx);
            let removed_obj = self.objects.remove(idx);
            self.layout.offsets.remove(idx);
            self.object_events.remove(idx);
            
            self.name_to_index.remove(&removed_name);
            self.id_to_index.remove(&removed_obj.id);
            
            for tag in &removed_obj.tags {
                if let Some(indices) = self.tag_to_indices.get_mut(tag) {
                    indices.retain(|&i| i != idx);
                }
            }
            
            for index in self.name_to_index.values_mut() {
                if *index > idx {
                    *index -= 1;
                }
            }
            
            for index in self.id_to_index.values_mut() {
                if *index > idx {
                    *index -= 1;
                }
            }
            
            for indices in self.tag_to_indices.values_mut() {
                for index in indices.iter_mut() {
                    if *index > idx {
                        *index -= 1;
                    }
                }
            }
        }
    }
    
    pub fn get_game_object(&self, name: &str) -> Option<&GameObject> {
        self.name_to_index.get(name)
            .and_then(|&idx| self.objects.get(idx))
    }
    
    pub fn get_game_object_mut(&mut self, name: &str) -> Option<&mut GameObject> {
        self.name_to_index.get(name).copied()
            .and_then(move |idx| self.objects.get_mut(idx))
    }
    
    fn check_collision(&self, idx1: usize, idx2: usize) -> bool {
        let obj1 = match self.objects.get(idx1) {
            Some(obj) => obj,
            None => return false,
        };
        let obj2 = match self.objects.get(idx2) {
            Some(obj) => obj,
            None => return false,
        };
        
        let obj1_right = obj1.position.0 + obj1.size.0;
        let obj1_bottom = obj1.position.1 + obj1.size.1;
        let obj2_right = obj2.position.0 + obj2.size.0;
        let obj2_bottom = obj2.position.1 + obj2.size.1;
        
        obj1.position.0 < obj2_right &&
        obj1_right > obj2.position.0 &&
        obj1.position.1 < obj2_bottom &&
        obj1_bottom > obj2.position.1
    }
    
    pub fn run(&mut self, action: Action) {
        match action {
            Action::ApplyMomentum { target, value } => {
                self.apply_to_targets(&target, |obj| {
                    obj.momentum.0 += value.0;
                    obj.momentum.1 += value.1;
                });
            }
            Action::SetMomentum { target, value } => {  
                self.apply_to_targets(&target, |obj| {
                    obj.momentum.0 = value.0;
                    obj.momentum.1 = value.1;
                });
            }
            Action::SetResistance { target, value } => {
                self.apply_to_targets(&target, |obj| {
                    obj.resistance = value;
                });
            }
            Action::Remove { target } => {
                let names = self.get_target_names(&target);
                for name in names {
                    self.remove_game_object(&name);
                }
            }
            Action::Spawn { object, location } => {
                let position = location.resolve_position(self);
                
                let mut new_obj = *object;
                new_obj.position = position;
                let name = format!("spawned_{}", new_obj.id);
                self.add_game_object(name, new_obj);
            }
        }
    }
    
    pub fn add_event(&mut self, event: GameEvent, target: Target) {
        let indices = self.get_target_indices(&target);
        for idx in indices {
            if let Some(events) = self.object_events.get_mut(idx) {
                events.push(event.clone());
            }
        }
    }
    
    fn trigger_collision_events(&mut self, idx: usize) {
        if let Some(events) = self.object_events.get(idx).cloned() {
            for event in events {
                if let GameEvent::Collision { action, target: _ } = event {
                    self.run(action);
                }
            }
        }
    }
    
    fn trigger_boundary_collision_events(&mut self, idx: usize) {
        if let Some(events) = self.object_events.get(idx).cloned() {
            let mut actions_to_run = Vec::new();
            for event in events {
                if let GameEvent::BoundaryCollision { action, target: _ } = event {
                    actions_to_run.push(action);
                }
            }
            
            for action in actions_to_run {
                self.run(action);
            }
        }
    }
    
    fn apply_to_targets<F>(&mut self, target: &Target, mut f: F)
    where
        F: FnMut(&mut GameObject),
    {
        let indices = self.get_target_indices(target);
        for idx in indices {
            if let Some(obj) = self.objects.get_mut(idx) {
                f(obj);
            }
        }
    }
    
    fn get_target_indices(&self, target: &Target) -> Vec<usize> {
        match target {
            Target::ByName(name) => {
                self.name_to_index.get(name)
                    .map(|&idx| vec![idx])
                    .unwrap_or_else(Vec::new)
            }
            Target::ById(id) => {
                self.id_to_index.get(id)
                    .map(|&idx| vec![idx])
                    .unwrap_or_else(Vec::new)
            }
            Target::ByTag(tag) => {
                self.tag_to_indices.get(tag).cloned().unwrap_or_else(Vec::new)
            }
        }
    }
    
    fn get_target_names(&self, target: &Target) -> Vec<String> {
        let indices = self.get_target_indices(target);
        indices.iter()
            .filter_map(|&idx| self.object_names.get(idx))
            .cloned()
            .collect()
    }
    
    pub fn collision_between(&self, target1: &Target, target2: &Target) -> bool {
        let indices1 = self.get_target_indices(target1);
        let indices2 = self.get_target_indices(target2);
        
        for &idx1 in &indices1 {
            for &idx2 in &indices2 {
                if idx1 != idx2 && self.check_collision(idx1, idx2) {
                    return true;
                }
            }
        }
        
        false
    }
}

impl Location {
    fn resolve_position(&self, canvas: &Canvas) -> (f32, f32) {
        match self {
            Location::Position(pos) => *pos,
            Location::AtTarget(target) => {
                if let Some(idx) = canvas.get_target_indices(target).first() {
                    if let Some(obj) = canvas.objects.get(*idx) {
                        obj.position
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                }
            }
            Location::Between(target1, target2) => {
                let pos1 = canvas.get_target_indices(target1).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position)
                    .unwrap_or((0.0, 0.0));
                let pos2 = canvas.get_target_indices(target2).first()
                    .and_then(|&idx| canvas.objects.get(idx))
                    .map(|obj| obj.position)
                    .unwrap_or((0.0, 0.0));
                ((pos1.0 + pos2.0) / 2.0, (pos1.1 + pos2.1) / 2.0)
            }
        }
    }
}


