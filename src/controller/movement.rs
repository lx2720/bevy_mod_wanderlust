use crate::controller::*;
use crate::spring::Strength;

/// Movements applied via inputs.
///
/// This includes directional movement and jumping.
use bevy_rapier3d::prelude::*;

/// Settings used to determine movement impulses on this controller.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Movement {
    /// How fast the controller will get to the `max_speed`.
    pub acceleration: Strength,

    /// How fast our controller will move.
    pub max_speed: f32,
    /// Scales movement force. This is useful to ensure movement does not
    /// affect vertical velocity (by setting it to e.g. `Vec3(1.0, 0.0, 1.0)`).
    pub force_scale: ForceScale,
    /// Scales movement force when we are slipping.
    /// If this is not `Vec3(1.0, 1.0, 1.0)` then the character can try to
    /// move up the slope.
    pub slip_force_scale: Vec3,
}

#[derive(Debug, Default, Clone, Reflect)]
pub enum ForceScale {
    /// Use the inverse of `Gravity::up_vector` for a force scale.
    #[default]
    Up,
    /// Don't scale at all.
    None,
    /// Specific force scale.
    Vec3(Vec3),
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            acceleration: Strength::Scaled(10.0),
            max_speed: 10.0,
            force_scale: default(),
            slip_force_scale: Vec3::splat(1.0),
        }
    }
}

impl Movement {
    pub fn force_scale(&self, gravity: &Gravity) -> Vec3 {
        match self.force_scale {
            ForceScale::Vec3(v) => v,
            ForceScale::Up => {
                if gravity.up_vector.length() > 0.0 {
                    let up = gravity.up_vector.normalize();
                    let (x, z) = up.any_orthonormal_pair();
                    x.abs() + z.abs()
                } else {
                    Vec3::ONE
                }
            }
            ForceScale::None => Vec3::ONE,
        }
    }
}

/// Calculated impulse for moving the character.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct MovementForce {
    /// Linear impulse to apply to move the character.
    pub linear: Vec3,
    /// Angular impulse to apply to move the character.
    pub angular: Vec3,
}

pub trait Cap {
    fn cap(self, cap: Self) -> Self;
}

impl Cap for Vec3 {
    fn cap(self, cap: Self) -> Self {
        let mut out = self;
        if cap.x == 0.0 ||
            cap.x < 0.0 && self.x < cap.x
            || cap.x > 0.0 && self.x > cap.x
        {
            out.x = cap.x;
        }
        if cap.y == 0.0 ||
            cap.y < 0.0 && self.y < cap.y
            || cap.y > 0.0 && self.y > cap.y
        {
            out.y = cap.y;
        }
        if cap.z == 0.0 ||
            cap.z < 0.0 && self.z < cap.z
            || cap.z > 0.0 && self.z > cap.z
        {
            out.z = cap.z;
        }

        out
    }
}

/// Calculates the movement forces for this controller.
pub fn movement_force(
    ctx: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut MovementForce,
        &mut Movement,
        &Gravity,
        &ControllerInput,
        &GroundCast,
        &GroundCaster,
        &ControllerVelocity,
        &ControllerMass,
    )>,
    frictions: Query<&Friction>,
    mut gizmos: Gizmos,
) {
    let dt = ctx.integration_parameters.dt;
    for (
        controller_entity,
        global,
        mut force,
        movement,
        gravity,
        input,
        cast,
        ground_caster,
        velocity,
        mass,
    ) in &mut query
    {
        force.linear = Vec3::ZERO;

        let force_scale = movement.force_scale(&gravity);

        let input_dir = input.movement.clamp_length_max(1.0);
        let input_goal_vel = input_dir * movement.max_speed;
        let mut goal_vel = input_goal_vel;

        let slip_force = match cast.current {
            Some(ground) if !ground.stable => {
                let (x, z) = ground.cast.normal.any_orthonormal_pair();
                gizmos.ray(ground.cast.point, ground.cast.normal, Color::BLUE);

                let projected_x = gravity.up_vector.project_onto(x);
                let projected_z = gravity.up_vector.project_onto(z);
                let slip_vector = (projected_x + projected_z) * force_scale;
                gizmos.ray(ground.cast.point, slip_vector, Color::LIME_GREEN);

                // arbitrary value to ignore flat surfaces.
                if slip_vector.length() > 0.01 {
                    // Counteract the movement going up the slope.
                    let slip_goal = goal_vel
                        .project_onto(slip_vector.normalize())
                        .max(Vec3::ZERO);
                    goal_vel -= slip_goal * movement.slip_force_scale;

                    // Pushing to force the controller down the slope
                    let slide = (slip_vector * force_scale).normalize();
                    Some(slide)
                } else {
                    None
                }
            }
            _ => None,
        };

        let last_ground_vel = if let Some(ground) = cast.viable.last() {
            ground.point_velocity
        } else {
            Velocity::default()
        };

        let relative_velocity = (velocity.linear - last_ground_vel.linvel) * force_scale;
        let friction_force = if let ViableGround::Ground(ground) = cast.viable {
            let friction = frictions
                .get(controller_entity)
                .copied()
                .unwrap_or(Friction::default());
            let ground_friction = frictions
                .get(ground.entity)
                .copied()
                .unwrap_or(Friction::default());
            let friction_coefficient = friction.coefficient.max(ground_friction.coefficient);
            friction_coefficient * relative_velocity * mass.mass / dt
        } else {
            Vec3::ZERO
        };
        //info!("friction: {:.2?}", friction_force.length());
        //info!("relative velocity: {:.2?}", (relative_velocity * force_scale).length());

        // Debug check to make sure we can clamp by the instant force
        //assert!((-instant_force).cmple(instant_force).all());

        let strength = movement.acceleration.get(mass.mass, dt);

        // This is effectively an implicit spring-damper function since the displacement is the velocity.
        // We could try to add a damping factor here based off acceleration, but I'm not sure it matters.
        let mut movement_force = (input_goal_vel * strength * force_scale);

        // get displacement of relative velocity to goal velocity
        let clamped_velocity = relative_velocity.cap(goal_vel);

        let displacement = goal_vel - clamped_velocity;
        let max_movement_force = displacement * mass.mass / dt * force_scale + friction_force;
        let movement_force = movement_force.cap(max_movement_force);

        if movement_force.length() > 0.1 {
            info!("displacement: {:.1?}", displacement);
            info!("relative_vel: {:.1?}", relative_velocity);
            info!("goal_vel: {:.1?}", goal_vel);
            info!("movement_force: {:.1?}", movement_force);
            info!("max_movement_force: {:.1?}", max_movement_force);
        }

        force.linear += movement_force - friction_force - slip_force.unwrap_or(Vec3::ZERO);
    }
}

/// How the controller's jumping should behave.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Jump {
    /// The amount of force to apply on the first frame when a jump begins.
    pub initial_force: f32,
    /// The amount of force to continuously apply every second during a jump.
    pub force: f32,
    /// How long to wait before we can jump again.
    pub cooldown_duration: f32,
    /// Timer for tracking `cooldown_duration`.
    pub cooldown_timer: f32,
    /// How long a jump can last.
    pub jump_duration: f32,
    /// Timer for tracking `jump_duration`.
    pub jump_timer: f32,
    /// A function taking the current progress of a jump, from 0.0 to 1.0, with 0.0 indicating a jump has just begun and 1.0 indicating the jump has ended,
    /// which returns a modifier (usually from 0.0 to 1.0, but not necessarily) to multiply [`jump_force`](ControllerSettings::jump_force) by.
    #[reflect(ignore)]
    pub decay_function: Option<fn(f32) -> f32>,

    /// Number of times we can jump before we have to touch the ground again.
    pub jumps: u32,
    /// Remaining before we have to touch the ground again.
    pub remaining_jumps: u32,
    /// Was [`ControllerInput::jumping`] true last frame.
    pub pressed_last_frame: bool,
    /// The amount of force to apply downwards when the jump control is released prior to a jump expiring.
    /// This allows analog jumping by cutting the jump short when the control is released.
    pub stop_force: f32,

    /// A timer to track jump buffering. See [`jump_buffer_duration`](ControllerSettings::jump_buffer_duration)
    pub buffer_timer: f32,
    /// If the jump input is pressed before landing, how long will the jump be buffered for?
    /// In other words, if this is 0.5, the character can input jump up to 0.5 seconds
    /// before landing and the jump will occur when they land.
    pub buffer_duration: f32,

    /// Do we have to be grounded to jump for the first time?
    pub first_jump_grounded: bool,
    /// How long should the character still be able to jump after leaving the ground, in seconds.
    /// For example, if this is set to 0.5, the player can fall off a ledge and then jump if they do so within 0.5 seconds of leaving the ledge.
    pub coyote_duration: f32,
    /// A timer to track coyote time. See [`coyote_duration`](Self::coyote_duration)
    pub coyote_timer: f32,

    /// How long to skip ground checks after jumping. Usually this should be set just high enough that the character is out of range of the ground
    /// just before the timer elapses.
    pub skip_ground_check_duration: f32,
}

impl Default for Jump {
    fn default() -> Self {
        Self {
            initial_force: 30.0,
            force: 20.0,
            cooldown_duration: 0.25,
            cooldown_timer: 0.0,
            jump_duration: 0.1,
            jump_timer: 0.0,
            decay_function: Some(|x| (1.0 - x).sqrt()),
            stop_force: 0.3,

            buffer_duration: 0.3,
            buffer_timer: 0.0,

            first_jump_grounded: true,
            coyote_duration: 0.2,
            coyote_timer: 0.0,

            jumps: 1,
            remaining_jumps: 1,
            pressed_last_frame: false,

            skip_ground_check_duration: 0.3,
        }
    }
}

impl Jump {
    /// Tick down timers by `dt`/delta time.
    pub fn tick_timers(&mut self, dt: f32) {
        let tick = |timer: &mut f32| {
            if *timer > 0.0 {
                *timer = (*timer - dt).max(0.0);
            }
        };

        tick(&mut self.cooldown_timer);
        tick(&mut self.jump_timer);
        tick(&mut self.buffer_timer);
        tick(&mut self.coyote_duration);
    }

    /// Are we currently jumping?
    pub fn jumping(&self) -> bool {
        self.jump_timer > 0.0
    }

    /// Can we jump right now?
    pub fn can_jump(&self, grounded: bool) -> bool {
        let first_jump = self.remaining_jumps == self.jumps;
        //info!("first_jump: {:?}", first_jump);
        let grounded = grounded || self.coyote_timer > 0.0;
        //info!("grounded: {:?}", grounded);
        if first_jump && !grounded {
            return false;
        }

        self.cooldown_timer <= 0.0 && self.remaining_jumps > 0
    }

    /// Reset the jumping state.
    pub fn reset_jump(&mut self) {
        self.remaining_jumps = self.jumps;
        self.jump_timer = 0.0;
    }

    /// 0..1 progress of the current jump.
    pub fn jump_progress(&self) -> f32 {
        (self.jump_duration - self.jump_timer) / self.jump_duration
    }

    /// Jump force decay multiplier.
    pub fn decay_multiplier(&self) -> f32 {
        if let Some(decay_function) = self.decay_function {
            (decay_function)(self.jump_progress())
        } else {
            1.0
        }
    }
}

/// Calculated force for controller jumping.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct JumpForce {
    /// Linear impulse to apply to push the character up.
    pub linear: Vec3,
}

/// Calculate the jump force for the controller.
pub fn jump_force(
    mut query: Query<(
        &mut JumpForce,
        &mut FloatForce,
        &mut GravityForce,
        &mut Jump,
        &ControllerInput,
        &mut GroundCaster,
        &GroundCast,
        &Grounded,
        &Gravity,
        &ControllerVelocity,
        &ControllerMass,
    )>,
    ctx: Res<RapierContext>,
) {
    let dt = ctx.integration_parameters.dt;
    for (
        mut force,
        mut float_force,
        mut gravity_force,
        mut jumping,
        input,
        mut ground_caster,
        ground_cast,
        grounded,
        gravity,
        velocity,
        mass,
    ) in &mut query
    {
        force.linear = Vec3::ZERO;

        let grounded = ground_cast.grounded();
        jumping.tick_timers(dt);

        if grounded {
            jumping.coyote_timer = jumping.coyote_duration;
        }

        if jumping.cooldown_timer <= 0.0 && grounded {
            jumping.reset_jump();
        }

        let velocity = if let Some(ground) = ground_cast.viable.last() {
            velocity.linear - ground.point_velocity.linvel
        } else {
            velocity.linear
        };

        let jump_inputted = input.jumping && !jumping.pressed_last_frame;

        let just_jumped = jump_inputted || jumping.buffer_timer > 0.0;

        if jump_inputted && !grounded {
            jumping.buffer_timer = jumping.buffer_duration;
        }

        if jumping.can_jump(grounded) && just_jumped {
            // Negating the current velocity increases consistency for falling jumps,
            // and prevents stacking jumps to reach high upwards velocities
            let initial_jump_force = jumping.initial_force * gravity.up_vector;
            let negate_up_velocity =
                (-1.0 * gravity.up_vector * velocity.dot(gravity.up_vector)) * mass.mass / dt;
            force.linear += negate_up_velocity + initial_jump_force;

            gravity_force.linear = Vec3::ZERO;
            float_force.linear = Vec3::ZERO;

            jumping.remaining_jumps = jumping.remaining_jumps.saturating_sub(1);
            jumping.cooldown_timer = jumping.cooldown_duration;

            jumping.jump_timer = jumping.jump_duration;
        // don't double up on initial force and jumping forces.
        } else if jumping.jumping() {
            if !input.jumping {
                // Cut the jump short if we aren't holding the jump down.
                //jumping.reset_jump();
                let stop_force = velocity.project_onto(gravity.up_vector) * -jumping.stop_force;
                force.linear += stop_force;
            } else {
                ground_caster.skip_ground_check_timer = jumping.skip_ground_check_duration;

                let jump = gravity.up_vector * jumping.force * jumping.decay_multiplier();
                force.linear += jump;
            }
        }

        jumping.pressed_last_frame = input.jumping;
    }
}
