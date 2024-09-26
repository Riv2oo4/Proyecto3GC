mod framebuffer;
mod ray_intersect;
mod cube;
mod color;
mod camera;
mod light;
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

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Object],
    yellow_light_position: &Vec3, 
    depth: u32,
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
        return SKYBOX_COLOR;
    }

    let light_dir = (yellow_light_position - intersect.point).normalize();
    let view_dir = (ray_origin - intersect.point).normalize();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();

    let shadow_intensity = cast_shadow(&intersect, yellow_light_position, objects);
    let light_intensity = 1.5 * (1.0 - shadow_intensity);

    let diffuse_intensity = intersect.normal.dot(&light_dir).max(0.0).min(1.0);
    let diffuse = intersect.material.diffuse * intersect.material.albedo[0] * diffuse_intensity * light_intensity;

    let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.specular);
    let specular = Color::new(255, 255, 255) * intersect.material.albedo[1] * specular_intensity * light_intensity;

    diffuse + specular
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Object], camera: &Camera, yellow_light_position: &Vec3) {
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

            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, yellow_light_position, 0);

            framebuffer.set_current_color(pixel_color.to_hex());
            framebuffer.point(x, y);
        }
    }
}

// Animación de agua en una cuadrícula
fn generate_wave_grid(
    water_material: Material, 
    grid_size: usize, 
    cube_size: f32, 
    elapsed_time: f32
) -> Vec<Object> {
    let mut water_cubes = Vec::new();
    for x in 0..grid_size {
        for z in 0..grid_size {
            let wave_height = (elapsed_time * 2.0 + (x as f32 + z as f32) * 0.5).sin() * 0.2; // Simulación de ondas
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

// Generar borde de arena alrededor del agua
fn generate_sand_border(sand_material: Material, grid_size: usize, cube_size: f32) -> Vec<Object> {
    let mut sand_cubes = Vec::new();
    for x in 0..grid_size {
        for z in 0..grid_size {
            // Crear bordes de arena en el contorno
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

// Generar casita de arena
fn generate_sand_house(sand_material: Material, start_position: Vec3, cube_size: f32) -> Vec<Object> {
    let mut house_cubes = Vec::new();

    let house_width = 5;  // Anchura de la casa (cubos)
    let house_height = 3;  // Altura de la casa (cubos)
    let house_depth = 5;  // Profundidad de la casa (cubos)

    // Construimos las paredes y el techo
    for x in 0..house_width {
        for y in 0..house_height {
            for z in 0..house_depth {
                // Dejar huecos para puertas y ventanas
                let is_door = (x == 2 && z == 0 && y < 2);  // Puerta en el frente
                let is_window = (y == 1 && (x == 1 || x == 3) && (z == 0 || z == house_depth - 1));  // Ventanas
                
                if !(is_door || is_window) {  // No poner cubo si es puerta o ventana
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

    // Techo de la casa (cubriendo toda la superficie)
    for x in 0..house_width {
        for z in 0..house_depth {
            house_cubes.push(Object::Cube(
                Cube {
                    center: Vec3::new(
                        start_position.x + x as f32 * cube_size,
                        start_position.y + house_height as f32 * cube_size,  // Techo está en la altura máxima
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

    // Material para la arena
    let sand_color = Material::new(
        Color::new(237, 201, 175), 
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    // Material para el tronco de la palmera
    let brown_trunk = Material::new(
        Color::new(139, 69, 19),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    // Material para las hojas de la palmera
    let green_leaf = Material::new(
        Color::new(34, 139, 34),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    // Material para el oasis (agua)
    let water_material = Material::new(
        Color::new(0, 191, 255),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    // Creamos los objetos de la escena: terreno de arena, palmera y oasis
    let mut objects = vec![
        // Terreno (cubos grandes planos)
        Object::Cube(Cube { center: Vec3::new(0.0, 0.0, 0.0), size: 10.0, material: sand_color }, false),
    ];

    // Añadimos el tronco de la palmera
    let trunk_start_y = 5.0;  // El tronco comienza en la parte superior del cubo de arena (altura 5.0)
    let trunk_cube_size = 0.4;
    let num_trunk_cubes = 5;  // Número de cubos para el tronco

    for i in 0..num_trunk_cubes {
        objects.push(Object::Cube(Cube { 
            center: Vec3::new(0.0, trunk_start_y + i as f32 * trunk_cube_size, 0.0),  
            size: trunk_cube_size, 
            material: brown_trunk 
        }, false));
    }

    // Hojas de la palmera
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

    // Inicializamos la animación
    let start_time = Instant::now();  // Usamos el tiempo para animar el agua

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

        // Calculamos el tiempo transcurrido
        let elapsed_time = start_time.elapsed().as_secs_f32();
        
        // Generamos la cuadrícula de agua animada
        let water_grid = generate_wave_grid(water_material, 6, 0.5, elapsed_time);  // Cambia el tamaño de la cuadrícula y de los cubos

        // Generamos los bordes de arena alrededor del agua
        let sand_border = generate_sand_border(sand_color, 6, 0.5);

        // Generamos una casita de arena
        let sand_house = generate_sand_house(sand_color, Vec3::new(-4.5, 5.2, -4.0), 0.5);

        // Actualizamos la lista de objetos con el agua, los bordes de arena, y la casa
        let mut objects_with_water_and_house = objects.clone();
        objects_with_water_and_house.extend(water_grid);
        objects_with_water_and_house.extend(sand_border);
        objects_with_water_and_house.extend(sand_house);  // Añadimos la casita

        if window.is_key_down(Key::W) {
            camera.move_camera("forward"); 
        }
    
        if window.is_key_down(Key::S) {
            camera.move_camera("backward");
        }
    
        if window.is_key_down(Key::A) {
            camera.move_camera("left"); 
        }
    
        if window.is_key_down(Key::D) {
            camera.move_camera("right"); 
        }
    
        if window.is_key_down(Key::Left) {
            camera.orbit(rotation_speed, 0.0); 
        }
    
        if window.is_key_down(Key::Right) {
            camera.orbit(-rotation_speed, 0.0); 
        }
    
        if window.is_key_down(Key::Up) {
            camera.orbit(0.0, -rotation_speed); 
        }
    
        if window.is_key_down(Key::Down) {
            camera.orbit(0.0, rotation_speed); 
        }
    
        render(&mut framebuffer, &objects_with_water_and_house, &camera, &yellow_light_position);
    
        window
            .update_with_buffer(&framebuffer.buffer, framebuffer.width, framebuffer.height)
            .unwrap();
    
        std::thread::sleep(frame_delay);
    }
}
