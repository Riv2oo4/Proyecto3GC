use crate::color::Color;
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub diffuse: Color,
    pub specular: f32,
    pub albedo: [f32; 4],
    pub refractive_index: f32,
    pub emission: Color, // Color de la emisión
    pub is_emissive: bool, // Indica si el material es emisivo
}

impl Material {
    pub fn new(
        diffuse: Color,
        specular: f32,
        albedo: [f32; 4],
        refractive_index: f32,
        emission: Color, // Añadimos el color de emisión
        is_emissive: bool, // Indicador de si es emisivo o no
    ) -> Self {
        Material {
            diffuse,
            specular,
            albedo,
            refractive_index,
            emission,
            is_emissive,
        }
    }

    // Método para crear un material no emisivo
    pub fn black() -> Self {
        Material {
            diffuse: Color::new(0, 0, 0),
            specular: 0.0,
            albedo: [0.0, 0.0, 0.0, 0.0],
            refractive_index: 0.0,
            emission: Color::black(), // Sin emisión
            is_emissive: false,
        }
    }
}
