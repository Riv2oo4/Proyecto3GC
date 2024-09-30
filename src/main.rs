mod framebuffer;
mod ray_intersect;
mod cube;
mod color;
mod camera;
mod material;

use minifb::{Window, WindowOptions, Key};
use nalgebra_glm::{Vec3, normalize};
use std::time::{Duration, Instant}; 
use std::f32::consts::PI;

use crate::color::Color;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::cube::Cube;
use crate::framebuffer::Framebuffer;
use crate::camera::Camera;
use crate::material::Material;

const ORIGIN_BIAS: f32 = 1e-4;
const SKYBOX_COLOR: Color = Color::new(68, 142, 228);

#[derive(Clone)]
enum Object {
    Cube(Cube, bool),
}

fn offset_origin(intersect: &Intersect, direction: &Vec3) -> Vec3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(&intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

fn cast_shadow(
    intersect: &Intersect,
    light_position: &Vec3,
    objects: &[Object],
) -> f32 {
    let light_dir = (light_position - intersect.point).normalize();
    let light_distance = (light_position - intersect.point).magnitude();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);
    let mut shadow_intensity = 0.0;

    for object in objects {
        let shadow_intersect = match object {
            Object::Cube(cube, _) => cube.ray_intersect(&shadow_ray_origin, &light_dir),
        };
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            let distance_ratio = shadow_intersect.distance / light_distance;
            shadow_intensity = 1.0 - distance_ratio.powf(2.0).min(1.0);
            break;
        }
    }

    shadow_intensity
}

fn interpolate_color(color1: Color, color2: Color, factor: f32) -> Color {
    Color::new(
        (color1.red() as f32 * (1.0 - factor) + color2.red() as f32 * factor) as u8,
        (color1.green() as f32 * (1.0 - factor) + color2.green() as f32 * factor) as u8,
        (color1.blue() as f32 * (1.0 - factor) + color2.blue() as f32 * factor) as u8,
    )
}

fn calculate_light_intensity(light_position: &Vec3) -> f32 {
    let max_intensity = 1.0;  
    let min_intensity = 0.2;  

    let light_height_factor = (light_position.y + 1.0).max(0.0) / 10.0;  

    min_intensity + (max_intensity - min_intensity) * light_height_factor.clamp(0.0, 1.0)
}


fn skybox_color(ray_direction: &Vec3, light_intensity: f32) -> Color {
    let t = 0.5 * (ray_direction.y + 1.0);  

    let sky_color_day = Color::new(135, 206, 235);  
    let ground_color_day = Color::new(222, 184, 135);  

    let sky_color_night = Color::new(25, 25, 112);  
    let ground_color_night = Color::new(50, 50, 50);  

    let sky_color = interpolate_color(sky_color_night, sky_color_day, light_intensity);
    let ground_color = interpolate_color(ground_color_night, ground_color_day, light_intensity);

    let blended_color = Color::new(
        ((1.0 - t) * ground_color.red() as f32 + t * sky_color.red() as f32) as u8,
        ((1.0 - t) * ground_color.green() as f32 + t * sky_color.green() as f32) as u8,
        ((1.0 - t) * ground_color.blue() as f32 + t * sky_color.blue() as f32) as u8,
    );

    blended_color
}
fn fresnel(cos_theta: f32, refractive_index: f32) -> f32 {
    let r0 = ((1.0 - refractive_index) / (1.0 + refractive_index)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5)
}
pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Object],
    light_positions: &[Vec3],
    depth: u32,
    light_intensity: f32,
) -> Color {
    if depth > 3 {
        return SKYBOX_COLOR;
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = match object {
            Object::Cube(cube, _) => cube.ray_intersect(ray_origin, ray_direction),
        };
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return skybox_color(ray_direction, light_intensity);
    }

    let mut total_diffuse = Color::black();
    let mut total_specular = Color::black();

    for light_position in light_positions {
        let light_dir = (light_position - intersect.point).normalize();
        let view_dir = (ray_origin - intersect.point).normalize();
        let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();

        let shadow_intensity = cast_shadow(&intersect, light_position, objects);
        let light_intensity = 1.5 * (1.0 - shadow_intensity);

        let cos_theta = -ray_direction.dot(&intersect.normal).max(0.0);
        let fresnel_effect = fresnel(cos_theta, intersect.material.refractive_index);

        let diffuse_intensity = intersect.normal.dot(&light_dir).max(0.0).min(1.0);
        total_diffuse = total_diffuse
            + (intersect.material.diffuse * intersect.material.albedo[0] * diffuse_intensity * light_intensity);

        let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.specular);
        total_specular = total_specular
            + (Color::new(255, 255, 255) * intersect.material.albedo[1] * specular_intensity * light_intensity * fresnel_effect);
    }

    let emission = if intersect.material.is_emissive {
        intersect.material.emission
    } else {
        Color::black()
    };

    total_diffuse + total_specular + emission
}



pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Object],
    camera: &Camera,
    light_positions: &[Vec3],  
    light_intensity: f32,  
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let rotated_direction = camera.base_change(&ray_direction);

            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, light_positions, 0, light_intensity);

            framebuffer.set_current_color(pixel_color.to_hex());
            framebuffer.point(x, y);
        }
    }
}


fn generate_wave_grid(
    water_material: Material, 
    grid_size: usize, 
    cube_size: f32, 
    elapsed_time: f32
) -> Vec<Object> {
    let mut water_cubes = Vec::new();
    for x in 0..grid_size {
        for z in 0..grid_size {
            let wave_height = (elapsed_time * 2.0 + (x as f32 + z as f32) * 0.5).sin() * 0.2; 
            water_cubes.push(Object::Cube(
                Cube {
                    center: Vec3::new(x as f32 * cube_size, 4.9 + wave_height, z as f32 * cube_size),
                    size: cube_size,
                    material: water_material,
                },
                false,
            ));
        }
    }
    water_cubes
}

fn generate_sand_border(sand_material: Material, grid_size: usize, cube_size: f32) -> Vec<Object> {
    let mut sand_cubes = Vec::new();
    for x in 0..grid_size {
        for z in 0..grid_size {
            if x == 0 || x == grid_size - 1 || z == 0 || z == grid_size - 1 {
                sand_cubes.push(Object::Cube(
                    Cube {
                        center: Vec3::new(x as f32 * cube_size, 4.9, z as f32 * cube_size),
                        size: cube_size,
                        material: sand_material,
                    },
                    false,
                ));
            }
        }
    }
    sand_cubes
}

fn generate_sand_house(sand_material: Material, start_position: Vec3, cube_size: f32) -> Vec<Object> {
    let mut house_cubes = Vec::new();

    let house_width = 5;  
    let house_height = 3;  
    let house_depth = 5;  

    for x in 0..house_width {
        for y in 0..house_height {
            for z in 0..house_depth {
                let is_door = x == 2 && z == 0 && y < 2;  
                let is_window = y == 1 && (x == 1 || x == 3) && (z == 0 || z == house_depth - 1);  
                
                if !(is_door || is_window) {  
                    house_cubes.push(Object::Cube(
                        Cube {
                            center: Vec3::new(
                                start_position.x + x as f32 * cube_size,
                                start_position.y + y as f32 * cube_size,
                                start_position.z + z as f32 * cube_size,
                            ),
                            size: cube_size,
                            material: sand_material,
                        },
                        false,
                    ));
                }
            }
        }
    }

    for x in 0..house_width {
        for z in 0..house_depth {
            house_cubes.push(Object::Cube(
                Cube {
                    center: Vec3::new(
                        start_position.x + x as f32 * cube_size,
                        start_position.y + house_height as f32 * cube_size,  
                        start_position.z + z as f32 * cube_size,
                    ),
                    size: cube_size,
                    material: sand_material,
                },
                false,
            ));
        }
    }

    house_cubes
}

fn main() {
    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);

    let mut window = Window::new(
        "Refractor",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();


    let sand_color = Material::new(
        Color::new(237, 201, 175),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Color::black(),  
        false,           
    );
    
    let brown_trunk = Material::new(
        Color::new(139, 69, 19),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Color::black(),  
        false,           
    );
    
    let green_leaf = Material::new(
        Color::new(34, 139, 34),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Color::black(),  
        false,           
    );
    
    let water_material = Material::new(
        Color::new(0, 191, 255),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Color::black(),  
        false,           
    );
    
    let light_cube_material = Material::new(
        Color::black(),              
        0.0,                        
        [0.0, 0.0, 0.0, 0.0],        
        0.0,                         
        Color::new(255, 223, 0),      
        true                         
    );
    
    let mut objects = vec![
        Object::Cube(Cube { center: Vec3::new(0.0, 0.0, 0.0), size: 10.0, material: sand_color }, false),
        Object::Cube(Cube { center: Vec3::new(1.0, 5.2, -4.0), size: 0.5, material: light_cube_material }, true),  
        Object::Cube(Cube { center: Vec3::new(4.5, 5.2, 2.0), size: 0.5, material: light_cube_material }, true),  
    ];

    let trunk_start_y = 5.0;  
    let trunk_cube_size = 0.4;
    let num_trunk_cubes = 5;  

    for i in 0..num_trunk_cubes {
        objects.push(Object::Cube(Cube { 
            center: Vec3::new(0.0, trunk_start_y + i as f32 * trunk_cube_size, 0.0),  
            size: trunk_cube_size, 
            material: brown_trunk 
        }, false));
    }

    let leaf_start_y = trunk_start_y + num_trunk_cubes as f32 * trunk_cube_size; 
    let leaf_positions = vec![
        Vec3::new(0.0, leaf_start_y, 0.0),
        Vec3::new(0.5, leaf_start_y, 0.5),
        Vec3::new(-0.5, leaf_start_y, 0.5),
        Vec3::new(0.5, leaf_start_y, -0.5),
        Vec3::new(-0.5, leaf_start_y, -0.5),
    ];

    for pos in leaf_positions {
        objects.push(Object::Cube(Cube { center: pos, size: 0.5, material: green_leaf }, false));
    }

    let start_time = Instant::now();  

    let mut camera = Camera::new(
        Vec3::new(5.0, 5.0, 10.0), 
        Vec3::new(0.0, 2.0, 0.0),  
        Vec3::new(0.0, 1.0, 0.0),  
    );

    let mut angle: f32 = 0.0;
    let radius = 15.0;
    let rotation_speed = 0.05;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        angle += rotation_speed; 
        
        let yellow_light_position = Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
        let light_positions = vec![
            Vec3::new(1.0, 5.2, -4.0),  
            Vec3::new(4.5, 5.2, 2.0),   
            yellow_light_position,       
        ];
    
        let light_intensity = calculate_light_intensity(&yellow_light_position);

        let elapsed_time = start_time.elapsed().as_secs_f32();
        
        let water_grid = generate_wave_grid(water_material, 6, 0.5, elapsed_time);  
    
        let sand_border = generate_sand_border(sand_color, 6, 0.5);
    
        let sand_house = generate_sand_house(sand_color, Vec3::new(-4.5, 5.2, -4.0), 0.5);
    
        let mut objects_with_water_and_house = objects.clone();
        objects_with_water_and_house.extend(water_grid);
        objects_with_water_and_house.extend(sand_border);
        objects_with_water_and_house.extend(sand_house);  
    
      if window.is_key_down(Key::W) {
        camera.move_camera("forward"); 
    }

    if window.is_key_down(Key::S) {
        camera.move_camera("backward");
    }

    if window.is_key_down(Key::A) {
        camera.orbit(rotation_speed, 0.0);  
    }

    if window.is_key_down(Key::D) {
        camera.orbit(-rotation_speed, 0.0);  
    }

    if window.is_key_down(Key::Up) {
        camera.orbit(0.0, -rotation_speed);  
    }

    if window.is_key_down(Key::Down) {
        camera.orbit(0.0, rotation_speed);  
    }

    if window.is_key_down(Key::Left) {
        camera.move_camera("left");  
    }

    if window.is_key_down(Key::Right) {
        camera.move_camera("right");  
    }
    
        render(&mut framebuffer, &objects_with_water_and_house, &camera, &light_positions, light_intensity);
    
        window
            .update_with_buffer(&framebuffer.buffer, framebuffer.width, framebuffer.height)
            .unwrap();
    
        std::thread::sleep(frame_delay);
    }
    
}
