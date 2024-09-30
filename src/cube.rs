use nalgebra_glm::Vec3;
use crate::ray_intersect::{RayIntersect, Intersect};
use crate::material::Material;

#[derive(Clone)]
pub struct Cube {
    pub center: Vec3,
    pub size: f32,
    pub material: Material,
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let half_size = self.size / 2.0;
        let min_bound = self.center - Vec3::new(half_size, half_size, half_size);
        let max_bound = self.center + Vec3::new(half_size, half_size, half_size);

        let mut t_min = (min_bound.x - ray_origin.x) / ray_direction.x;
        let mut t_max = (max_bound.x - ray_origin.x) / ray_direction.x;

        if t_min > t_max {
            std::mem::swap(&mut t_min, &mut t_max);
        }

        let mut t_y_min = (min_bound.y - ray_origin.y) / ray_direction.y;
        let mut t_y_max = (max_bound.y - ray_origin.y) / ray_direction.y;

        if t_y_min > t_y_max {
            std::mem::swap(&mut t_y_min, &mut t_y_max);
        }

        if (t_min > t_y_max) || (t_y_min > t_max) {
            return Intersect::empty();
        }

        if t_y_min > t_min {
            t_min = t_y_min;
        }

        if t_y_max < t_max {
            t_max = t_y_max;
        }

        let mut t_z_min = (min_bound.z - ray_origin.z) / ray_direction.z;
        let mut t_z_max = (max_bound.z - ray_origin.z) / ray_direction.z;

        if t_z_min > t_z_max {
            std::mem::swap(&mut t_z_min, &mut t_z_max);
        }

        if (t_min > t_z_max) || (t_z_min > t_max) {
            return Intersect::empty();
        }

        if t_z_min > t_min {
            t_min = t_z_min;
        }

        if t_min < 0.0 {
            return Intersect::empty();
        }

        let point = ray_origin + ray_direction * t_min;
        let mut normal = Vec3::new(0.0, 0.0, 0.0);

        if (point.x - min_bound.x).abs() < 1e-4 {
            normal = Vec3::new(-1.0, 0.0, 0.0);
        } else if (point.x - max_bound.x).abs() < 1e-4 {
            normal = Vec3::new(1.0, 0.0, 0.0);
        } else if (point.y - min_bound.y).abs() < 1e-4 {
            normal = Vec3::new(0.0, -1.0, 0.0);
        } else if (point.y - max_bound.y).abs() < 1e-4 {
            normal = Vec3::new(0.0, 1.0, 0.0);
        } else if (point.z - min_bound.z).abs() < 1e-4 {
            normal = Vec3::new(0.0, 0.0, -1.0);
        } else if (point.z - max_bound.z).abs() < 1e-4 {
            normal = Vec3::new(0.0, 0.0, 1.0);
        }

        let distance = t_min;
        Intersect::new(point, normal, distance, self.material.clone())  
    }
}
