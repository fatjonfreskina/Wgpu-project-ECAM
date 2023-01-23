// TODO:
// issue: https://github.com/gfx-rs/naga/issues/1490

// STILL TO DO ----------------------------------------------------------------------------------------------------------------------------------

// Ajouter l'atténuation des ressorts

// BONUS : forces de frottement mdr jamais de la vie


// IMPORTS --------------------------------------------------------------------------------------------------------------------------------------

use cgmath::num_traits::pow;
use wgpu_bootstrap::{
    window::Window,
    frame::Frame,
    cgmath::{ self },
    application::Application,
    // texture::create_texture_bind_group,
    context::Context,
    camera::Camera,
    default::{ Vertex, Particle },
    computation::Computation,
    geometry::{ icosphere },
    wgpu,
};


// CONSTANTS ------------------------------------------------------------------------------------------------------------------------------------

// how to increase number of instances : increase number of instances per row here AND increase workgroup size in compute.
// Nombre de particules par ligne
const NUM_INSTANCES_PER_ROW: u32 = 31;

// Ajustement de la position de départ des partcules pour les centrer et les remonter
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 - 1.0 , -35.0, NUM_INSTANCES_PER_ROW as f32 - 1.0);
const DIST_INTERVAL: f32 = 2.0; // distance entre les particules

// Rayon et centre de la sphère
const RADIUS: f32 = 25.0; 
const SPHERE_CENTER: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 0.0, 0.0);

// Masse des particules
const PART_MASS: f32 = 1.0; 

// longueur des ressorts
const STRUC_REST: f32 = DIST_INTERVAL;
const SHEAR_REST: f32 = DIST_INTERVAL * 1.41421356237;
const BEND_REST: f32 = DIST_INTERVAL * 2.0;

// constante de raideur des ressorts
const STRUC_STIFF: f32 = 300.0;
const SHEAR_STIFF: f32 = 10.0;
const BEND_STIFF: f32 = 10.0;

// constante d'atténuation des ressorts
// const STRUC_DAMP: f32 = 1.0;
// const SHEAR_DAMP: f32 = 1.0;
// const BEND_DAMP: f32 = 1.0;


// STRUCTS --------------------------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeData {
    delta_time: f32, // intervalle entre les calculs
    nb_instances: u32, // nombre de particules
    sphere_center_x: f32, // centre de la sphère (x)
    sphere_center_y: f32, // centre de la sphère (y)
    sphere_center_z: f32, // centre de la sphère (z)
    radius: f32, // rayon de la sphère
    part_mass: f32, // masse des particules
    struc_rest: f32, // longueur des ressorts structurels
    shear_rest: f32, // longueur des ressorts de cisaillement
    bend_rest: f32, // longueur des ressorts de flexion
    struc_stiff: f32, // constante de raideur des ressorts structurels
    shear_stiff: f32, // constante de raideur des ressorts de cisaillement
    bend_stiff: f32, // constante de raideur des ressorts de flexion
    // struc_damp: f32, // constante d'atténuation des ressorts structurels
    // shear_damp: f32, // constante d'atténuation des ressorts de cisaillement
    // bend_damp: f32, // constante d'atténuation des ressorts de flexion
}

struct Net {
    // diffuse_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
    particle_pipeline: wgpu::RenderPipeline,
    sphere_pipeline: wgpu::RenderPipeline, // line -> sphere
    compute_pipeline: wgpu::ComputePipeline, // added!
    compute_springs_pipeline: wgpu::ComputePipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    sphere_vertex_buffer: wgpu::Buffer, // sphere
    sphere_index_buffer: wgpu::Buffer, // sphere
    particles: Vec<Particle>,
    particle_buffer: wgpu::Buffer,
    compute_particles_bind_group: wgpu::BindGroup, // added!
    compute_springs_bind_group: wgpu::BindGroup,
    compute_data_buffer: wgpu::Buffer, // added!
    compute_data_bind_group: wgpu::BindGroup, // added!
    indices: Vec<u16>,
    sphere_indices: Vec<u16>, // added for sphere
}


// INFO --------------------------------------------------------------------------------------------------------------------------------------

// On crée un pipeline, qui prend des données dans un buffer.
// Les bindgroups sont des groupes de données qui sont liées à un pipeline et indiquent à ce pipeline où aller chercher les données.





impl Net {
    fn new(context: &Context) -> Self {
        

        // CAMERA ----------------------------------------------------------------------------------------------------------------------------

        let camera = Camera {
            eye: (20.0, 40.0, 100.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: context.get_aspect_ratio(),
            fovy: 45.0,
            znear: 0.1,
            zfar: 1200.0,
        };

        let (_camera_buffer, camera_bind_group) = camera.create_camera_bind_group(context);
    

        // PARTICULES ------------------------------------------------------------------------------------------------------------------------
        
        // création du pipeline (1) pour afficher les particules
        let particle_pipeline = context.create_render_pipeline(
            "Particle pipeline",
            include_str!("red.wgsl"),
            &[Vertex::desc(), Particle::desc()],
            &[
                &context.camera_bind_group_layout,
            ],
            wgpu::PrimitiveTopology::TriangleList
        );

        // génération des "balles" qui réprésentent les particules
        let (vertices, indices) = icosphere(1);
        // buffer pour les balles (un pour les vertex et un pour les indices)
        let vertex_buffer = context.create_buffer(vertices.as_slice(), wgpu::BufferUsages::VERTEX);
        let index_buffer = context.create_buffer(indices.as_slice(), wgpu::BufferUsages::INDEX);

        // création des particules elles-mêmes
        let particles = (0..NUM_INSTANCES_PER_ROW*NUM_INSTANCES_PER_ROW).map(|index| {
            let x = index % NUM_INSTANCES_PER_ROW;
            let z = index / NUM_INSTANCES_PER_ROW;
            // note : on multiplie par DIST_INTERVAL pour que les particules soient espacées de la distance spécifiée en haut
            let position = cgmath::Vector3 { x: x as f32*DIST_INTERVAL, y: 0.0, z: z as f32*DIST_INTERVAL } - INSTANCE_DISPLACEMENT;

            Particle {
                position: position.into(), 
                velocity: [0.0, 0.0, 0.0],
            }
        }).collect::<Vec<_>>();
        // buffer pour les particules
        let particle_buffer = context.create_buffer(particles.as_slice(), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE);
        

        // SPHERE ----------------------------------------------------------------------------------------------------------------------------

        // création du pipeline (2) pour afficher la sphere
        let sphere_pipeline = context.create_render_pipeline( // définit le pipeline pour le sphere
            "Sphere Pipeline",
            include_str!("blue.wgsl"),
            &[Vertex::desc()],
            &[
                &context.camera_bind_group_layout,
            ],
            wgpu::PrimitiveTopology::LineList,
        );
    
        // création de la sphere
        let (mut sphere_vertices, sphere_indices) = icosphere(4);
        
        // agrandir la sphere :
        for vertex in sphere_vertices.iter_mut() {
            let mut posn = cgmath::Vector3::from(vertex.position);
            posn *= RADIUS;
            vertex.position = posn.into()
        }
        
        // buffers pour la sphere (un pour les vertex et un pour les indices)
        let sphere_vertex_buffer = context.create_buffer(&sphere_vertices, wgpu::BufferUsages::VERTEX);
        let sphere_index_buffer = context.create_buffer(&sphere_indices, wgpu::BufferUsages::INDEX);


        // COMPUTE ---------------------------------------------------------------------------------------------------------------------------

        // création du pipeline (3) pour calculer le déplacement des particules
        let compute_pipeline = context.create_compute_pipeline("Compute Pipeline", include_str!("compute.wgsl"));

        // Bind group pour le calcul des particules (utilise le buffer de particules)
        let compute_particles_bind_group = context.create_bind_group(
            "Compute particles bind group", 
            &compute_pipeline.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding()
                }
            ]
        );

        // définit compute data (paramètres restent fixes à priori)
        let compute_data = ComputeData {
            delta_time: 0.016,
            nb_instances: pow(NUM_INSTANCES_PER_ROW,2),
            sphere_center_x: SPHERE_CENTER.x,
            sphere_center_y: SPHERE_CENTER.y,
            sphere_center_z: SPHERE_CENTER.z,
            radius: RADIUS,
            part_mass: PART_MASS,
            struc_rest: STRUC_REST,
            shear_rest: SHEAR_REST,
            bend_rest: BEND_REST,
            struc_stiff: STRUC_STIFF,
            shear_stiff: SHEAR_STIFF,
            bend_stiff: BEND_STIFF,
            // struc_damp: STRUC_DAMP,
            // shear_damp: f32,
            // bend_damp: f32,
        };

        // buffer pour compute data
        let compute_data_buffer = context.create_buffer(&[compute_data], wgpu::BufferUsages::UNIFORM);

        // Bind group pour compute data
        let compute_data_bind_group = context.create_bind_group(
            "Compute data bind group", 
            &compute_pipeline.get_bind_group_layout(1), 
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute_data_buffer.as_entire_binding(),
                }
            ]
        );


        // RESSORTS --------------------------------------------------------------------------------------------------------------------------

        // pipeline (4) pour calculer les ressorts structurels
        let compute_springs_pipeline = context.create_compute_pipeline(
            "spring compute pipeline", 
            include_str!("springs.wgsl"));

        // listes des ressorts
        let mut structural = Vec::new();
        let mut shear = Vec::new();
        let mut bend = Vec::new();

        for index in 0..particles.len() as i32 {
            // structural springs
            let row = index as u32 / NUM_INSTANCES_PER_ROW;
            let col = index as u32 % NUM_INSTANCES_PER_ROW;
            for offset in [-1,1] {
                // col -1 & +1
                if col as i32 + offset >= 0 && col as i32 + offset < NUM_INSTANCES_PER_ROW as i32 {
                    structural.push([index, index + offset]);
                } else {
                    structural.push([index, pow(NUM_INSTANCES_PER_ROW, 2) as i32 + 1]);
                }
                // row -1 & +1
                if row as i32 + offset >= 0 && row as i32 + offset < NUM_INSTANCES_PER_ROW as i32 {
                    structural.push([index, index + (offset * NUM_INSTANCES_PER_ROW as i32)]);
                } else {
                    structural.push([index, pow(NUM_INSTANCES_PER_ROW, 2) as i32 + 1]);
                }
            }
            // shear srings
            for offset1 in [-1,1] {
                for offset2 in [-1,1] {
                    if col as i32 + offset1 >= 0 && col as i32 + offset1 < NUM_INSTANCES_PER_ROW as i32 && row as i32 + offset2 >= 0 && row as i32 + offset2 < NUM_INSTANCES_PER_ROW as i32 {
                        shear.push([index, index + (offset2 * NUM_INSTANCES_PER_ROW as i32) + offset1]); 
                    } else {
                        shear.push([index, pow(NUM_INSTANCES_PER_ROW, 2) as i32 + 1]);
                    }
                }
            }
            // bend springs
            for offset in [-2,2] {
                // col -2 & +2
                if col as i32 + offset >= 0 && col as i32 + offset < NUM_INSTANCES_PER_ROW as i32 {
                    bend.push([index, index + offset]);
                } else {
                    bend.push([index, pow(NUM_INSTANCES_PER_ROW, 2) as i32 + 1]);
                }
                // row -2 & +2
                if row as i32 + offset >= 0 && row as i32 + offset < NUM_INSTANCES_PER_ROW as i32 {
                    bend.push([index, index + (offset * NUM_INSTANCES_PER_ROW as i32)]);
                } else {
                    bend.push([index, pow(NUM_INSTANCES_PER_ROW, 2) as i32 + 1]);
                }
            }
        }

        // Imprime les ressorts structurels (pour vérifier)
        // for elem in structural.iter_mut() {
        //     println!("{:?}", elem)
        // }
        // Imprime les ressorts de cisaillement (pour vérifier)
        // for elem in shear.iter_mut() {
        //     println!("{:?}", elem)
        // }
        // Impression des ressorts de flexion (pour vérifier)
        // for elem in bend.iter_mut() {
        //     println!("{:?}", elem)
        // }

        // buffer pour les ressorts structurels
        let structural_index_buffer = context.create_buffer(structural.as_slice(), wgpu::BufferUsages::STORAGE);
        let shear_index_buffer = context.create_buffer(shear.as_slice(), wgpu::BufferUsages::STORAGE);
        let bend_index_buffer = context.create_buffer(bend.as_slice(), wgpu::BufferUsages::STORAGE);

        // Bind group pour les ressorts
        let compute_springs_bind_group = context.create_bind_group(
            "Compute Springs Bind Group!", 
            &compute_pipeline.get_bind_group_layout(2),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: structural_index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: shear_index_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bend_index_buffer.as_entire_binding()
                },
            ]
        );

        Self {
            // diffuse_bind_group,
            camera_bind_group,
            particle_pipeline,
            sphere_pipeline,
            compute_pipeline,
            compute_springs_pipeline,
            vertex_buffer,
            index_buffer,
            sphere_vertex_buffer,
            sphere_index_buffer,
            particles,
            particle_buffer,
            compute_particles_bind_group,
            compute_springs_bind_group,
            compute_data_buffer,
            compute_data_bind_group,
            indices, 
            sphere_indices,
        }
    }
}





impl Application for Net {
    fn render(&self, context: &Context) -> Result<(), wgpu::SurfaceError> {
        let mut frame = Frame::new(context)?;

        {
            let mut render_pass = frame.begin_render_pass(wgpu::Color {r: 0.85, g: 0.85, b: 0.85, a: 1.0});
            
            // afficher les particules
            render_pass.set_pipeline(&self.particle_pipeline); // pipeline (1)
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // vertex buffer (pour les icosphères)
            render_pass.set_vertex_buffer(1, self.particle_buffer.slice(..)); // vertex buffer (pour les particules)
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..(self.indices.len() as u32), 0, 0..self.particles.len() as _);
            
            // render la sphere
            render_pass.set_pipeline(&self.sphere_pipeline); // pipeline (2)
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sphere_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.sphere_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.sphere_indices.len() as u32, 0, 0..1);
        }

        frame.present();

        Ok(())
    }
    
    fn update(&mut self, context: &Context, delta_time: f32) {
        // WHY COMPUTE_DATA ICI AUSSI ?
        let compute_data = ComputeData {
            delta_time: 0.016, // delta_time, on peut ne pas mettre : 0.016 si on veut utiliser le clock de l'ordi! (ce que je peux faire)
            nb_instances: pow(NUM_INSTANCES_PER_ROW,2),
            sphere_center_x: SPHERE_CENTER.x,
            sphere_center_y: SPHERE_CENTER.y,
            sphere_center_z: SPHERE_CENTER.z,
            radius: RADIUS,
            part_mass: PART_MASS,
            struc_rest: STRUC_REST,
            shear_rest: SHEAR_REST,
            bend_rest: BEND_REST,
            struc_stiff: STRUC_STIFF,
            shear_stiff: SHEAR_STIFF,
            bend_stiff: BEND_STIFF,
            // struc_damp: STRUC_DAMP,
            // shear_damp: f32,
            // bend_damp: f32,
        };
        context.update_buffer(&self.compute_data_buffer, &[compute_data]);

        //MISE A JOUR VIA LE COMPUTE SHADER
        let mut computation = Computation::new(context);

        {
            let mut compute_pass = computation.begin_compute_pass();

            // Calcul des forces de ressorts
            compute_pass.set_pipeline(&self.compute_springs_pipeline); // pipeline (3)
            compute_pass.set_bind_group(0, &self.compute_particles_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_data_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_springs_bind_group, &[]);
            compute_pass.dispatch_workgroups((pow(NUM_INSTANCES_PER_ROW,2) as f64/64.0).ceil() as u32, 1, 1);
            
            // Calcul des nouvelles positions
            compute_pass.set_pipeline(&self.compute_pipeline); // pipeline (4)
            compute_pass.set_bind_group(0, &self.compute_particles_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_data_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_springs_bind_group, &[]);
            compute_pass.dispatch_workgroups((pow(NUM_INSTANCES_PER_ROW,2) as f64/64.0).ceil() as u32, 1, 1);
        }

        computation.submit();

    }

    

}

fn main() {
    let window = Window::new();

    let context = window.get_context();

    let my_app = Net::new(context);

    window.run(my_app);
}
