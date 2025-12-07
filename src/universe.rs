use glam::{DVec3, Vec3};

// Real astronomical constants (SI units)
const G: f64 = 6.67430e-11;
const M_SUN: f64 = 1.989e30;
const M_EARTH: f64 = 5.972e24;
const M_MOON: f64 = 7.348e22;

const AU: f64 = 1.496e11; // Distance Earth-Sun
const LD: f64 = 3.844e8;  // Distance Earth-Moon

struct Body {
    pos: DVec3,
    vel: DVec3,
    mass: f64,
}

pub struct Universe {
    sun: Body,
    earth: Body,
    moon: Body,
    
    // Earth Rotation
    earth_rotation_angle: f64, // Radians
    day_duration: f64, // Seconds for one full rotation (86400)
}

impl Universe {
    pub fn new() -> Self {
        // Initialize roughly correct starting positions/velocities
        // Simplified: Circular orbits for stability in this toy model
        
        // Sun at origin (approx inertial center)
        let sun = Body {
            pos: DVec3::ZERO,
            vel: DVec3::ZERO,
            mass: M_SUN,
        };

        // Earth at 1 AU
        // Orbital velocity v = sqrt(GM / r)
        let v_earth = (G * M_SUN / AU).sqrt();
        let earth = Body {
            pos: DVec3::new(AU, 0.0, 0.0),
            vel: DVec3::new(0.0, 0.0, v_earth),
            mass: M_EARTH,
        };

        // Moon relative to Earth
        // Orbiting Earth
        let v_moon_rel = (G * M_EARTH / LD).sqrt();
        let moon = Body {
            pos: earth.pos + DVec3::new(LD, 0.0, 0.0),
            vel: earth.vel + DVec3::new(0.0, 0.0, v_moon_rel), // Orbiting Earth in same plane? 
            // Real moon orbit is tilted ~5 deg to ecliptic. Let's tilt it slightly for visual variation.
            // Tilt by 5 degrees around X axis
            mass: M_MOON,
        };
        
        // Apply tilt to Moon position/velocity relative to Earth
        // Actually, let's keep it simple planar for now, or tilt the whole Moon orbit plane.
        // Let's just let gravity handle it.
        
        Self {
            sun,
            earth,
            moon,
            earth_rotation_angle: 0.0,
            day_duration: 86400.0,
        }
    }

    pub fn step(&mut self, dt: f64) {
        // 1. Calculate Gravity Forces (Newton)
        // Force on Earth from Sun
        let r_se = self.sun.pos - self.earth.pos;
        let dist_se = r_se.length();
        let f_se = (G * self.sun.mass * self.earth.mass / (dist_se * dist_se)) * r_se.normalize();

        // Force on Moon from Earth
        let r_em = self.earth.pos - self.moon.pos;
        let dist_em = r_em.length();
        let f_em = (G * self.earth.mass * self.moon.mass / (dist_em * dist_em)) * r_em.normalize();

        // Force on Moon from Sun (Perturbation)
        let r_sm = self.sun.pos - self.moon.pos;
        let dist_sm = r_sm.length();
        let f_sm = (G * self.sun.mass * self.moon.mass / (dist_sm * dist_sm)) * r_sm.normalize();

        // Update Velocities (a = F/m)
        // Earth
        let a_earth = (f_se - f_em) / self.earth.mass; // Newtons 3rd law: Earth pulled by Sun, and pulled by Moon
        self.earth.vel += a_earth * dt;

        // Moon
        let a_moon = (f_em + f_sm) / self.moon.mass;
        self.moon.vel += a_moon * dt;

        // Sun (Negligible but correctness)
        let a_sun = (-f_se - f_sm) / self.sun.mass;
        self.sun.vel += a_sun * dt;

        // Update Positions
        self.earth.pos += self.earth.vel * dt;
        self.moon.pos += self.moon.vel * dt;
        self.sun.pos += self.sun.vel * dt;

        // 2. Update Earth Rotation
        let rot_speed = 2.0 * std::f64::consts::PI / self.day_duration;
        self.earth_rotation_angle = (self.earth_rotation_angle + rot_speed * dt) % (2.0 * std::f64::consts::PI);
    }

    // Get Sun direction relative to an observer on Earth's surface
    // We assume observer is at "Lat/Lon (0,0)" for simplicity, or rotating with the earth.
    // The "Up" vector matches the player's Y.
    pub fn get_sky_state(&self) -> (Vec3, Vec3) {
        // Inertial Vectors
        let to_sun = (self.sun.pos - self.earth.pos).normalize();
        let to_moon = (self.moon.pos - self.earth.pos).normalize();

        // Earth Rotation Transform
        // We simulate the observer rotating around the Earth's axis.
        // Earth axis is tilted ~23.5 degrees (0.41 rad) relative to orbit normal (Y in our setup? No, Z was velocity).
        // Let's say Orbit is in XZ plane (Y is Up-Ecliptic).
        // Earth axis is tilted.
        
        let tilt = 23.5_f64.to_radians();
        
        // Rotate vectors INTO the observer's frame.
        // Equivalent to rotating the Universe opposite to Earth's rotation.
        
        // 1. Rotate for Day/Night (around Earth Axis)
        // Rotation around Earth's Axis (Let's say tilted Y).
        // Actually, easiest way: 
        // Define Earth Axis in Inertial space: Tilted Y.
        // Observer is rotating around this axis.
        // We apply a rotation matrix R(-angle) around the axis to the celestial vectors.
        
        let axis = DVec3::new(0.0, tilt.cos(), tilt.sin()).normalize(); // Tilted axis
        let angle = -self.earth_rotation_angle;
        
        let q = glam::DQuat::from_axis_angle(axis, angle);
        
        let local_sun = q * to_sun;
        let local_moon = q * to_moon;

        // 2. Align so that "up" is "out from surface".
        // If observer is at equator, the axis is perpendicular to Up.
        // If we just use the rotation above, "Y" in the result is the direction of the Axis? No.
        // We need to map the rotated frame to the Player's Horizon frame (Y is Up).
        // Let's assume observer is at Equator at the Prime Meridian (locally).
        // At angle=0, Sun is at Zenith?
        // Just returning the rotated vector is basically "View from center of transparent Earth".
        // That works for skybox direction.
        
        (local_sun.as_vec3(), local_moon.as_vec3())
    }
}
