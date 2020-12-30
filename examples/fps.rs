use miniquad::*;

use glam::{vec3, Mat4, Vec3};

#[derive(Default)]
struct Cam {
    pitch_deg: f32,
    yaw_deg: f32,
}

impl Cam {
    fn facing(&self) -> Vec3 {
        let (pitch, yaw) = (self.pitch_deg.to_radians(), self.yaw_deg.to_radians());
        vec3(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos())
    }

    fn turn(&mut self, pitch_delta_deg: f32, yaw_delta_deg: f32) {
        self.pitch_deg = (self.pitch_deg + pitch_delta_deg).max(-89.0).min(89.0);
        self.yaw_deg = (self.yaw_deg + yaw_delta_deg) % 360.0;
    }
}

const MAX_OBSTACLES: usize = 512 * 1024;
struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    obstacles: Vec<Vec3>,
    cam: Cam,
    pos: Vec3,
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Stage {
        let r = 0.3;
        #[rustfmt::skip]
        let vertices: &[f32] = &[
            // positions          colors
            0.0,   -r, 0.0,       1.0, 0.0, 0.0, 1.0,
               r, 0.0, r,         0.0, 1.0, 0.0, 1.0,
               r, 0.0, -r,        0.0, 0.0, 1.0, 1.0,
              -r, 0.0, -r,        1.0, 1.0, 0.0, 1.0,
              -r, 0.0, r,         0.0, 1.0, 1.0, 1.0,
             0.0,   r, 0.0,       1.0, 0.0, 1.0, 1.0
        ];
        // vertex buffer for static geometry
        let geometry_vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 2,    0, 2, 3,    0, 3, 4,    0, 4, 1,
            5, 1, 2,    5, 2, 3,    5, 3, 4,    5, 4, 1
        ];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        // empty, dynamic instance-data vertex buffer
        let positions_vertex_buffer = Buffer::stream(
            ctx,
            BufferType::VertexBuffer,
            MAX_OBSTACLES * std::mem::size_of::<Vec3>(),
        );

        let bindings = Bindings {
            vertex_buffers: vec![geometry_vertex_buffer, positions_vertex_buffer],
            index_buffer: index_buffer,
            images: vec![],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

        let pipeline = Pipeline::with_params(
            ctx,
            &[
                BufferLayout::default(),
                BufferLayout {
                    step_func: VertexStep::PerInstance,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::with_buffer("pos", VertexFormat::Float3, 0),
                VertexAttribute::with_buffer("color0", VertexFormat::Float4, 0),
                VertexAttribute::with_buffer("inst_pos", VertexFormat::Float3, 1),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            }
        );

        let mut obstacles = vec![];

        for x in 0..10 {
            for y in 0..10 {
                let p = vec3(x as f32, 0.0, y as f32);
                if p.length() > 0.0 {
                    obstacles.push(p);
                }
            }
        }

        Stage {
            pipeline,
            bindings,
            obstacles,
            pos: Vec3::zero(),
            cam: Default::default(),
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        ctx.set_cursor_grab(true);
        self.bindings.vertex_buffers[1].update(ctx, &self.obstacles[..]);
    }

    fn mouse_delta_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.cam.turn(y * 0.1, -x * 0.1);
    }

    fn draw(&mut self, ctx: &mut Context) {
        // model-view-projection matrix
        let (width, height) = ctx.screen_size();
        let proj = Mat4::perspective_rh_gl(45.0f32.to_radians(), width / height, 0.01, 50.0);
        let view = Mat4::look_at_rh(
            self.pos,
            self.pos + self.cam.facing(),
            vec3(0.0, 1.0, 0.0),
        );
        let view_proj = proj * view;

        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { view_proj });
        ctx.draw(0, 24, self.obstacles.len() as i32);
        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

fn main() {
    miniquad::start(conf::Conf::default(), |mut ctx| {
        UserData::owning(Stage::new(&mut ctx), ctx)
    });
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec3 pos;
    attribute vec4 color0;
    attribute vec3 inst_pos;

    varying lowp vec4 color;

    uniform mat4 view_proj;

    void main() {
        vec4 pos = vec4(pos + inst_pos, 1.0);
        gl_Position = view_proj * pos;
        color = color0;
    }
    "#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec4 color;
    
    void main() {
        gl_FragColor = color;
    }
    "#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec![],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("view_proj", UniformType::Mat4)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub view_proj: glam::Mat4,
    }
}

