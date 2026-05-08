use super::core::Canvas;
use crate::types::GravityFalloff;

/// Shared constant. An object at exactly planet_radius × GRAVITY_INFLUENCE_MULT
/// is at the edge of the gravity field and receives zero pull.
pub(crate) const GRAVITY_INFLUENCE_MULT: f32 = 3.0;

/// How strongly non-dominant planet forces are dampened when an object is deep
/// inside the dominant (closest) planet's gravity field.  At the dominant
/// planet's surface the dampening factor reaches this value (0.0–1.0).
/// 0.9 means non-dominant forces are reduced to 10 % at the surface.
pub(crate) const NESTED_GRAVITY_DAMPENING: f32 = 0.9;

/// Compute the gravitational force vector from one planet onto one receiver.
///
/// Returns Some((fx, fy, pull_magnitude)) when the planet is in range,
/// None when the planet is outside the influence field or distance is negligible.
pub(crate) fn compute_gravity_force(
    obj_cx:             f32,
    obj_cy:             f32,
    obj_strength:       f32,
    obj_influence_mult: f32,
    obj_falloff:        GravityFalloff,
    planet_cx:          f32,
    planet_cy:          f32,
    planet_radius:      f32,
    planet_strength:    f32,
) -> Option<(f32, f32, f32)> {
    let dx   = planet_cx - obj_cx;
    let dy   = planet_cy - obj_cy;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 { return None; }

    let field_radius = planet_radius * obj_influence_mult;
    if dist > field_radius { return None; }

    let pull = match obj_falloff {
        GravityFalloff::Linear => {
            obj_strength * planet_strength * planet_radius / dist
        }
        GravityFalloff::InverseSquare => {
            obj_strength * planet_strength * (planet_radius * planet_radius) / (dist * dist)
        }
    };

    let nx = dx / dist;
    let ny = dy / dist;
    Some((nx * pull, ny * pull, pull))
}

impl Canvas {
    pub(crate) fn apply_directional_gravity(&mut self) {
        if self.crystalline.is_some() { return; }

        struct PlanetSnapshot {
            id:       String,
            cx:       f32,
            cy:       f32,
            radius:   f32,
            strength: f32,
            tags:     Vec<String>,
        }
        let planets: Vec<PlanetSnapshot> = self.store.objects.iter().map(|obj| {
            let cx = obj.position.0 + obj.size.0 * 0.5;
            let cy = obj.position.1 + obj.size.1 * 0.5;
            PlanetSnapshot {
                id:       obj.id.clone(),
                cx, cy,
                radius:   obj.planet_radius.unwrap_or(0.0),
                strength: obj.gravity_strength,
                tags:     obj.tags.clone(),
            }
        })
        .filter(|p| p.radius > 0.0)
        .collect();

        struct GravityResult {
            idx:         usize,
            fx:          f32,
            fy:          f32,
            dominant_id: Option<String>,
        }
        let mut results: Vec<GravityResult> = Vec::new();

        for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.visible { continue; }

            let tag_filter: Option<&str> = if obj.gravity_all_sources {
                None
            } else {
                match &obj.gravity_target {
                    Some(t) => Some(t.as_str()),
                    None    => continue,
                }
            };

            let obj_strength       = obj.gravity_strength;
            let obj_influence_mult = obj.gravity_influence_mult;
            let obj_falloff        = obj.gravity_falloff;
            let obj_cx             = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy             = obj.position.1 + obj.size.1 * 0.5;

            let mut total_fx       = 0.0_f32;
            let mut total_fy       = 0.0_f32;
            let mut dominant_id: Option<String> = None;

            // -- First pass: collect per-planet forces and find dominant ------
            struct PlanetForce {
                planet_idx: usize,
                fx:  f32,
                fy:  f32,
                surface_dist: f32,
                planet_radius: f32,
            }
            let mut planet_forces: Vec<PlanetForce> = Vec::new();
            let mut dom_pf_idx: Option<usize> = None;
            let mut dom_surface_dist = f32::MAX;

            for (pi, planet) in planets.iter().enumerate() {
                if planet.radius <= 0.0 { continue; }

                if let Some(tag) = tag_filter {
                    if !planet.tags.iter().any(|t| t == tag) { continue; }
                }

                if let Some((fx, fy, _pull)) = compute_gravity_force(
                    obj_cx, obj_cy,
                    obj_strength, obj_influence_mult, obj_falloff,
                    planet.cx, planet.cy, planet.radius, planet.strength,
                ) {
                    let dx = obj_cx - planet.cx;
                    let dy = obj_cy - planet.cy;
                    let surface_dist = (dx * dx + dy * dy).sqrt() - planet.radius;
                    if surface_dist < dom_surface_dist {
                        dom_surface_dist = surface_dist;
                        dom_pf_idx = Some(planet_forces.len());
                    }
                    planet_forces.push(PlanetForce {
                        planet_idx: pi, fx, fy, surface_dist, planet_radius: planet.radius,
                    });
                }
            }

            // -- Second pass: accumulate with nested-field dampening ----------
            let dampening_mult = if let Some(di) = dom_pf_idx {
                let dom_r = planet_forces[di].planet_radius;
                let max_field_surf = dom_r * (obj_influence_mult - 1.0);
                if max_field_surf > 0.0 {
                    let depth = 1.0 - (dom_surface_dist / max_field_surf).clamp(0.0, 1.0);
                    1.0 - depth * NESTED_GRAVITY_DAMPENING
                } else {
                    1.0
                }
            } else {
                1.0
            };

            for (i, pf) in planet_forces.iter().enumerate() {
                if Some(i) == dom_pf_idx {
                    total_fx += pf.fx;
                    total_fy += pf.fy;
                    dominant_id = Some(planets[pf.planet_idx].id.clone());
                } else {
                    total_fx += pf.fx * dampening_mult;
                    total_fy += pf.fy * dampening_mult;
                }
            }

            results.push(GravityResult {
                idx: obj_idx,
                fx: total_fx,
                fy: total_fy,
                dominant_id,
            });
        }

        for result in results {
            let obj = &mut self.store.objects[result.idx];
            if result.fx != 0.0 || result.fy != 0.0 {
                obj.momentum.0 += result.fx;
                obj.momentum.1 += result.fy;
            }
            obj.gravity_dominant_id = result.dominant_id;
        }
    }

    pub(crate) fn handle_planet_landings(&mut self) {
        if self.crystalline.is_some() { return; }

        let planets: Vec<(String, f32, f32, f32)> = self.store.objects
            .iter()
            .filter_map(|obj| {
                let r  = obj.planet_radius?;
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                Some((obj.id.clone(), cx, cy, r))
            })
            .collect();

        let mut corrections: Vec<(usize, f32, f32, f32, f32)> = Vec::new();

        'outer: for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.visible || obj.planet_radius.is_some() { continue; }

            let obj_cx = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy = obj.position.1 + obj.size.1 * 0.5;

            // Try the dominant planet first — it is the physically correct landing
            // target when multiple planet surfaces are near each other.
            let dominant = obj.gravity_dominant_id.as_deref();
            let ordered: Vec<&(String, f32, f32, f32)> = {
                let mut v: Vec<&_> = planets.iter()
                    .filter(|(id, _, _, _)| Some(id.as_str()) == dominant)
                    .collect();
                v.extend(planets.iter()
                    .filter(|(id, _, _, _)| Some(id.as_str()) != dominant));
                v
            };

            for (_, planet_cx, planet_cy, radius) in ordered {
                let dx   = obj_cx - planet_cx;
                let dy   = obj_cy - planet_cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist >= *radius || dist < 0.01 { continue; }

                let nx = dx / dist;
                let ny = dy / dist;
                let new_pos_x = planet_cx + nx * radius - obj.size.0 * 0.5;
                let new_pos_y = planet_cy + ny * radius - obj.size.1 * 0.5;
                corrections.push((obj_idx, new_pos_x, new_pos_y, nx, ny));
                continue 'outer;
            }
        }

        for (obj_idx, new_x, new_y, nx, ny) in corrections {
            let obj = &mut self.store.objects[obj_idx];

            let inward = obj.momentum.0 * (-nx) + obj.momentum.1 * (-ny);
            if inward > 0.0 {
                obj.momentum.0 += nx * inward;
                obj.momentum.1 += ny * inward;
            }

            obj.position.0 = new_x;
            obj.position.1 = new_y;

            let adj = super::rotation_adjusted_offset(
                obj.position,
                obj.size,
                obj.rotation,
                obj.slope.is_some(),
                obj.pivot,
            );
            if let Some(offset) = self.layout.offsets.get_mut(obj_idx) {
                *offset = adj;
            }
        }
    }

    pub(crate) fn apply_auto_align(&mut self) {
        let planets_tagged: Vec<(String, Vec<String>, f32, f32, f32)> = self.store.objects
            .iter()
            .filter_map(|obj| {
                let r = obj.planet_radius?;
                let cx = obj.position.0 + obj.size.0 * 0.5;
                let cy = obj.position.1 + obj.size.1 * 0.5;
                Some((obj.id.clone(), obj.tags.clone(), cx, cy, r))
            })
            .collect();

        let mut adjustments: Vec<(usize, f32)> = Vec::new();

        for (obj_idx, obj) in self.store.objects.iter().enumerate() {
            if !obj.auto_align || !obj.visible { continue; }

            let obj_cx = obj.position.0 + obj.size.0 * 0.5;
            let obj_cy = obj.position.1 + obj.size.1 * 0.5;
            let speed  = obj.auto_align_speed;
            let thresh = obj.auto_align_threshold;
            let obj_influence_mult = obj.gravity_influence_mult;

            // Resolve target planet and its radius for depth scaling.
            // Primary: use gravity_dominant_id (set by gravity pass when in-field).
            // Fallback: nearest planet within the gravity field.
            let planet_info: Option<(f32, f32, f32)> = if let Some(dom_id) = &obj.gravity_dominant_id {
                planets_tagged.iter()
                    .find(|(id, _, _, _, _)| id == dom_id)
                    .map(|(_, _, pcx, pcy, pr)| (*pcx, *pcy, *pr))
            } else {
                None
            };

            let (planet_cx, planet_cy, planet_r) = match planet_info {
                Some(p) => p,
                None => {
                    // Fallback: find nearest planet within effective gravity range.
                    let nearest = if let Some(target_tag) = &obj.gravity_target {
                        planets_tagged.iter()
                            .filter(|(_, tags, _, _, _)| tags.iter().any(|t| t == target_tag))
                            .map(|(_, _, pcx, pcy, pr)| {
                                let dx = pcx - obj_cx;
                                let dy = pcy - obj_cy;
                                let dist = (dx * dx + dy * dy).sqrt();
                                (dist, *pcx, *pcy, *pr)
                            })
                            .filter(|(d, _, _, pr)| *d > 0.01 && *d <= *pr * obj_influence_mult)
                            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                    } else if obj.gravity_all_sources {
                        planets_tagged.iter()
                            .map(|(_, _, pcx, pcy, pr)| {
                                let dx = pcx - obj_cx;
                                let dy = pcy - obj_cy;
                                let dist = (dx * dx + dy * dy).sqrt();
                                (dist, *pcx, *pcy, *pr)
                            })
                            .filter(|(d, _, _, pr)| *d > 0.01 && *d <= *pr * obj_influence_mult)
                            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                    } else {
                        continue;
                    };
                    match nearest {
                        Some((_, pcx, pcy, pr)) => (pcx, pcy, pr),
                        None => continue,
                    }
                }
            };

            let dx = obj_cx - planet_cx;
            let dy = obj_cy - planet_cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let target_angle = dy.atan2(dx).to_degrees() + 90.0;
            let diff = shortest_angle_diff(obj.rotation, target_angle);

            if diff.abs() > thresh { continue; }

            // Scale alignment strength by how deep into the gravity field
            // the object is.  depth_raw goes from 0.0 at the field edge to
            // 1.0 at the planet surface.  auto_align_min_depth controls the
            // threshold: the object must be at least that fraction into the
            // field before any alignment applies.  Above the threshold the
            // strength ramps linearly to 1.0 at the surface.
            let field_radius = planet_r * obj_influence_mult;
            let min_depth = obj.auto_align_min_depth;
            let depth_factor = if field_radius > planet_r {
                let depth_raw = ((field_radius - dist) / (field_radius - planet_r)).clamp(0.0, 1.0);
                if depth_raw < min_depth {
                    0.0
                } else if min_depth >= 1.0 {
                    1.0
                } else {
                    (depth_raw - min_depth) / (1.0 - min_depth)
                }
            } else {
                1.0
            };

            let push = diff.signum() * speed.min(diff.abs()) * depth_factor;
            adjustments.push((obj_idx, push));
        }

        for (idx, push) in adjustments {
            self.store.objects[idx].rotation_momentum += push;
        }
    }
}

fn shortest_angle_diff(from: f32, to: f32) -> f32 {
    let diff = (to - from).rem_euclid(360.0);
    if diff > 180.0 { diff - 360.0 } else { diff }
}
