use bevy::ecs::reflect::ReflectComponent;
use bevy::math::vec3;
use bevy::prelude::{default, warn, Component, Entity, Vec3};
use bevy::reflect::Reflect;
use bevy::utils::HashSet;
use bevy_rapier3d::prelude::Collider;

use crate::Spring;

/// The settings of a character controller. See each individual field for more description.
///
/// The [`Default::default()`] of this type is not well configured; it is not a good reference for any character controller, and will not do much.
/// See bundles like [`CharacterControllerBundle`](super::bundles::CharacterControllerBundle) for well-config
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ControllerSettings {
    /// How quickly to interpolate from `last_goal_velocity` to the new `input_goal_velocity`.
    /// In other words, how quickly to go from "not moving" to "moving at max speed".
    pub acceleration: f32,
    /// The length of the calculated `input_goal_velocity`.
    /// In other words, the speed to attempt to reach if a movement input (such as forwards) is fully saturated.
    ///
    /// Keys are generally either not saturated or fully saturated, while analog controls like a joystick can be partially saturated (half tilt).
    pub max_speed: f32,
    /// The maximum amount of force that can be applied to fulfill [`acceleration`](ControllerSettings::acceleration).
    pub max_acceleration_force: f32,
    /// The direction to jump, which is also the direction that gravity is opposite to.
    pub up_vector: Vec3,
    /// The strength of gravity.
    pub gravity: f32,
    /// Scales movement force. This is useful to ensure movement does not affect vertical velocity (by setting it to e.g. `Vec3(1.0, 0.0, 1.0)`).
    pub force_scale: Vec3,
    /// Scaling factor for the impulse applied to the ground to keep the character moving/off the ground.
    pub opposing_impulse_scale: f32,
    /// Scaling factor for the movement impulse applied to the ground.
    /// Setting this to 0.0 would make it so things don't "slip" out from the characters feet.
    pub opposing_movement_impulse_scale: f32,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            acceleration: default(),
            max_speed: default(),
            max_acceleration_force: default(),
            up_vector: default(),
            forward_vector: default(),
            gravity: default(),
            max_ground_angle: default(),
            min_float_offset: default(),
            max_float_offset: default(),
            jump_initial_force: default(),
            jump_force: default(),
            jump_stop_force: default(),
            jump_time: 1.0,
            jump_decay_function: None,
            jump_skip_ground_check_duration: default(),
            skip_ground_check_override: default(),
            extra_jumps: default(),
            coyote_time_duration: default(),
            jump_buffer_duration: default(),
            force_scale: default(),
            float_cast_length: default(),
            float_cast_origin: default(),
            float_cast_collider: Collider::ball(1.0),
            float_distance: default(),
            float_spring: default(),
            upright_spring: default(),
            exclude_from_ground: default(),
            opposing_impulse_scale: 1.0,
            opposing_movement_impulse_scale: 1.0,
        }
    }
}

impl ControllerSettings {
    /// A basic preset for a standard, walking character controller. Works for most first and third person games.
    pub fn character() -> Self {
        ControllerSettings {
            acceleration: 50.0,
            max_speed: 10.0,
            max_acceleration_force: 10.0,
            up_vector: Vec3::Y,
            //gravity: -9.8,
            gravity: -20.0,
            max_ground_angle: 45.0 * (std::f32::consts::PI / 180.0),
            min_float_offset: -0.3,
            max_float_offset: 0.05,
            jump_time: 0.5,
            jump_initial_force: 15.0,
            jump_stop_force: 0.3,
            jump_decay_function: Some(|x| (1.0 - x).sqrt()),
            jump_skip_ground_check_duration: 0.5,
            coyote_time_duration: 0.16,
            jump_buffer_duration: 0.16,
            force_scale: vec3(1.0, 0.0, 1.0),
            float_cast_length: 1.0,
            float_cast_collider: Collider::ball(0.45),
            float_distance: 0.55,
            float_spring: Spring {
                strength: 100.0,
                damping: 0.8,
            },
            upright_spring: Spring {
                strength: 10.0,
                damping: 0.5,
            },
            ..default()
        }
    }

    /// A sample controller preset for a spaceship which can fly in any direction.
    pub fn starship() -> Self {
        ControllerSettings {
            acceleration: 0.3,
            max_speed: 100.0,
            max_acceleration_force: 10.0,
            up_vector: Vec3::Y,
            force_scale: vec3(1.0, 1.0, 1.0),
            upright_spring: Spring {
                strength: 0.0,
                damping: 0.0,
            },
            ..default()
        }
    }
}
