mod block;
mod camera;
mod cube;
mod error;
mod event;
mod shader;
mod text;
mod texture;
mod wire_cube;

use std::f32::consts::PI;

use image;
use cgmath::{ Point, Point3, Matrix4 };
use glium::{ self, glutin, DisplayBuild, Surface };
use glium::program::Program;
use glium::glutin::Event as GlEvent;
use glium::backend::glutin_backend::WinRef;

use self::camera::Camera;
use self::text::Text;
use self::error::RenderCreationError;
use self::event::Event;
use game::GameState;

const MOUSE_SENSIVITY: f32 = 0.1;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub cube_pos: (i32, i32, i32),
}
implement_vertex!(Position, cube_pos);


#[derive(Clone, Copy, Debug)]
pub enum HDirection {
    Forth,
    Back,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum VDirection {
    Up,
    Down,
}

pub struct Renderer {
    display: glium::Display,
    cube_program: Program,
    wire_program: Program,
    camera: Camera,
    fov: f32, //in radians
    text: Text,
    stats: bool,
    fill: bool,
    game: GameState,
}

impl Renderer {
    pub fn new() -> Result<Renderer, RenderCreationError<glutin::CreationError>> {
        let display = try!(glutin::WindowBuilder::new()
            .with_depth_buffer(24)
            .build_glium());
        let cube_prog = try!(Program::from_source(
            &display,
            shader::CUBE_VERTEX,
            shader::CUBE_FRAGMENT,
            None,
        ));
        let wire_prog = try!(Program::from_source(
            &display,
            shader::WIRE_VERTEX,
            shader::WIRE_FRAGMENT,
            None,
        ));
        let text = try!(Text::new(&display, "/usr/share/fonts/TTF/NotoSans-Regular.ttf", 24));
        {
            let window = display.get_window().unwrap();
            window.set_cursor(glium::glutin::MouseCursor::Crosshair);
        }
        Ok(Renderer {
            display: display,
            cube_program: cube_prog,
            wire_program: wire_prog,
            camera: Camera::at(Point3::new(5.0, 5.0, 5.0), Point::origin()),
            fov: PI / 3.0,
            text: text,
            stats: false,
            fill: true,
            game: GameState::new(),
        })
    }

    pub fn game_loop(mut self) {

        let image = image::load(::std::io::Cursor::new(&include_bytes!(
            "../../assets/textures/dirt.png"
        )[..]),image::PNG).unwrap().to_rgba();
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
        let texture = glium::texture::SrgbTexture2d::new(&self.display, image).unwrap();
        let texture_sampler = glium::uniforms::Sampler::new(&texture)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest);


        let vb_cube = glium::VertexBuffer::new(&self.display, &cube::VERTICES).unwrap();
        let ib_cube = glium::IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &cube::INDICES).unwrap();

        let vb_wire = glium::VertexBuffer::new(&self.display, &wire_cube::VERTICES).unwrap();
        let ib_wire = glium::IndexBuffer::new(&self.display, glium::index::PrimitiveType::LinesList, &wire_cube::INDICES).unwrap();

        loop {
            let mut target = self.display.draw();
            target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);
            //target.clear_depth(1.0);

            let perspective: [[f32; 4]; 4] = self.get_perspective(&target).into();
            let view: [[f32; 4]; 4] = self.camera.view_matrix().into();
            {
                let params = self.get_params();

                let cubes: Vec<_> = self.game.stones.into_iter().map(|x| Position { cube_pos: (0, 0, x as i32) }).collect();
                let cubes_buffer = glium::vertex::VertexBuffer::new(&self.display, &cubes).unwrap();

                let wires: Vec<Position> = self.game.stones.into_iter().min().map(|x| Position { cube_pos: (0, 0, x as i32) }).into_iter().collect();
                let wires_buffer = glium::vertex::VertexBuffer::new(&self.display, &wires).unwrap();


                target.draw(
                    (&vb_cube, cubes_buffer.per_instance().unwrap()),
                    &ib_cube,
                    &self.cube_program,
                    &uniform! {
                        model: block::MODEL,
                        view: view,
                        perspective: perspective,
                        tex: texture_sampler,
                    },
                    &params
                ).unwrap();
                target.draw(
                    (&vb_wire, wires_buffer.per_instance().unwrap()),
                    &ib_wire,
                    &self.wire_program,
                    &uniform! { model: block::MODEL, view: view, perspective: perspective },
                    &params
                ).unwrap();

                if self.stats {
                    self.text.draw(&mut target, &format!("{}", self.camera), (1.0, 1.0, 0.0, 1.0));
                }
            }
            target.finish().unwrap();

            if !self.handle_events() {
                return;
            }
            self.camera.update();
        }
    }

    fn get_perspective<T: Surface>(&self, surface: &T) -> Matrix4<f32> {
        let (width, height) = surface.get_dimensions();
        let aspect_ratio = height as f32 / width as f32;

        let zfar = 1024.0;
        let znear = 0.1;

        let f = 1.0 / (self.fov / 2.0).tan();

        Matrix4::new(
            f * aspect_ratio, 0.0,              0.0              , 0.0,
                   0.0      ,  f ,              0.0              , 0.0,
                   0.0      , 0.0,      (zfar+znear)/(zfar-znear), 1.0,
                   0.0      , 0.0, -(2.0*zfar*znear)/(zfar-znear), 0.0,
        )
    }

    fn get_params(&self) -> glium::DrawParameters {
        use glium::draw_parameters::PolygonMode::{ Fill, Line };
        use glium::draw_parameters::DepthTest;
        glium::DrawParameters {
            depth: glium::Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            //backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            polygon_mode: if self.fill { Fill } else { Line },
            .. Default::default()
        }
    }


    fn convert(ev: GlEvent) -> Option<Event> {
        use self::event::Event::*;
        use self::HDirection::*;
        use self::VDirection::*;
        use glium::glutin::Event as E;
        use glium::glutin::ElementState::Pressed;
        use glium::glutin::VirtualKeyCode as V;
        match ev {
            E::KeyboardInput(state, _, Some(key)) => {
                let t = state == Pressed;
                match (state, key) {
                    (Pressed, V::Key1)   => Some(ToogleBlock { block: 1 }),
                    (Pressed, V::Key2)   => Some(ToogleBlock { block: 2 }),
                    (Pressed, V::Key3)   => Some(ToogleBlock { block: 3 }),
                    (Pressed, V::Key4)   => Some(ToogleBlock { block: 4 }),
                    (Pressed, V::Key5)   => Some(ToogleBlock { block: 5 }),
                    (Pressed, V::Key6)   => Some(ToogleBlock { block: 6 }),
                    (Pressed, V::Key7)   => Some(ToogleBlock { block: 7 }),
                    (Pressed, V::Key8)   => Some(ToogleBlock { block: 8 }),
                    (Pressed, V::Key9)   => Some(ToogleBlock { block: 9 }),
                    (_      , V::W)      => Some(Move { dir: Forth, toogle: t }),
                    (_      , V::A)      => Some(Move { dir: Left , toogle: t }),
                    (_      , V::S)      => Some(Move { dir: Back , toogle: t }),
                    (_      , V::D)      => Some(Move { dir: Right, toogle: t }),
                    (_      , V::Up)     => Some(Turn { dir: Forth, toogle: t }),
                    (_      , V::Left)   => Some(Turn { dir: Left , toogle: t }),
                    (_      , V::Down)   => Some(Turn { dir: Back , toogle: t }),
                    (_      , V::Right)  => Some(Turn { dir: Right, toogle: t }),
                    (_      , V::Space)  => Some(Fly  { dir: Up   , toogle: t }),
                    (_      , V::LShift) => Some(Fly  { dir: Down , toogle: t }),
                    _ => None,
                }

            },
            _ => None,
        }
    }

    fn handle_events(&mut self) -> bool {
        for ev in self.display.poll_events() {
            use glium::glutin::Event as E;
            use glium::glutin::ElementState::Pressed;
            use glium::glutin::VirtualKeyCode::*;
            match ev {
                E::Closed => return false,
                E::MouseMoved((mouse_x, mouse_y)) => {
                    let window = self.display.get_window().unwrap();
                    let (mid_x, mid_y) = Renderer::fix_mouse(window);
                    // screen coordinates increase to the right, just like phi
                    self.camera.add_phi((mouse_x - mid_x as i32) as f32 * MOUSE_SENSIVITY);
                    // screen coordinates decrease to the top, unlike theta
                    self.camera.add_theta((mid_y - mouse_y as i32) as f32 * MOUSE_SENSIVITY);
                },
                E::KeyboardInput(Pressed, _, Some(Escape)) => return false,
                _ => if let Some(ev) = Renderer::convert(ev) {
                    use self::event::Event::*;
                    match ev {
                        ToogleBlock { block: n }   => self.game.flip_stone(n),
                        Move { dir: d, toogle: t } => self.camera.mov (d, t),
                        Turn { dir: d, toogle: t } => self.camera.turn(d, t),
                        Fly  { dir: d, toogle: t } => self.camera.fly (d, t),
                        _ => {}
                    }
                },
            }
        }
        return true;
    }

    //returns mouse delta
    fn fix_mouse(window: WinRef) -> (i32, i32) {
        let (x, y) = window.get_inner_size_points().unwrap();
        let (mid_x, mid_y) = (x as i32 / 2, y as i32 / 2);
        window.set_cursor_position(mid_x, mid_y).unwrap();
        (mid_x, mid_y)
    }
}
