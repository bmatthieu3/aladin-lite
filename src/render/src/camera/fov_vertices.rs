use cgmath::Matrix4;
use cgmath::{Vector2, Vector4};

use crate::sphere_geometry::FieldOfViewType;

pub type NormalizedDeviceCoord = Vector2<f32>;
pub type WorldCoord = Vector4<f32>;
pub type ModelCoord = Vector4<f32>;

fn ndc_to_world<P: Projection>(
    ndc_coo: &[NormalizedDeviceCoord],
    ndc_to_clip: &Vector2<f32>,
    clip_zoom_factor: f32,
    longitude_reversed: bool,
) -> Option<Vec<WorldCoord>> {
    // Deproject the FOV from ndc to the world space
    let mut world_coo = Vec::with_capacity(ndc_coo.len());
    let _out_of_fov = false;

    for n in ndc_coo {
        let c = Vector2::new(
            n.x * ndc_to_clip.x * clip_zoom_factor,
            n.y * ndc_to_clip.y * clip_zoom_factor,
        );
        let w = P::clip_to_world_space(&c, longitude_reversed);
        if let Some(w) = w {
            world_coo.push(w);
        } else {
            // out of fov
            return None;
        }
    }

    Some(world_coo)
}
fn world_to_model(world_coo: &[WorldCoord], mat: &Matrix4<f32>) -> Vec<ModelCoord> {
    let mut model_coo = Vec::with_capacity(world_coo.len());

    for w in world_coo.iter() {
        //let m = r.rotate(w);
        model_coo.push(mat * w);
    }

    model_coo
}

const NUM_VERTICES_WIDTH: usize = 10;
const NUM_VERTICES_HEIGHT: usize = 10;
const NUM_VERTICES: usize = 4 + 2 * NUM_VERTICES_WIDTH + 2 * NUM_VERTICES_HEIGHT;
// This struct belongs to the CameraViewPort
pub struct FieldOfViewVertices {
    ndc_coo: Vec<NormalizedDeviceCoord>,
    world_coo: Option<Vec<WorldCoord>>,
    model_coo: Option<Vec<ModelCoord>>,

    // Meridians and parallels contained
    // in the field of view
    great_circles: FieldOfViewType,
}

impl FieldOfViewVertices {
    pub fn new<P: Projection>(
        _center: &Vector4<f32>,
        ndc_to_clip: &Vector2<f32>,
        clip_zoom_factor: f32,
        mat: &Matrix4<f32>,
        longitude_reversed: bool,
    ) -> Self {
        let mut x_ndc =
            itertools_num::linspace::<f32>(-1., 1., NUM_VERTICES_WIDTH + 2).collect::<Vec<_>>();

        x_ndc.extend(iter::repeat(1_f32).take(NUM_VERTICES_HEIGHT));
        x_ndc.extend(itertools_num::linspace::<f32>(
            1.,
            -1.,
            NUM_VERTICES_WIDTH + 2,
        ));
        x_ndc.extend(iter::repeat(-1_f32).take(NUM_VERTICES_HEIGHT));

        let mut y_ndc = iter::repeat(-1_f32)
            .take(NUM_VERTICES_WIDTH + 1)
            .collect::<Vec<_>>();

        y_ndc.extend(itertools_num::linspace::<f32>(
            -1.,
            1.,
            NUM_VERTICES_HEIGHT + 2,
        ));
        y_ndc.extend(iter::repeat(1_f32).take(NUM_VERTICES_WIDTH));
        y_ndc.extend(itertools_num::linspace::<f32>(
            1.,
            -1.,
            NUM_VERTICES_HEIGHT + 2,
        ));
        y_ndc.pop();

        let mut ndc_coo = Vec::with_capacity(NUM_VERTICES);
        for idx_vertex in 0..NUM_VERTICES {
            ndc_coo.push(Vector2::new(x_ndc[idx_vertex], y_ndc[idx_vertex]));
        }

        let world_coo =
            ndc_to_world::<P>(&ndc_coo, ndc_to_clip, clip_zoom_factor, longitude_reversed);
        let model_coo = if let Some(world_coo) = &world_coo {
            Some(world_to_model(world_coo, mat))
        } else {
            None
        };

        let great_circles = if let Some(vertices) = &model_coo {
            FieldOfViewType::new_polygon(vertices)
        } else {
            FieldOfViewType::new_allsky()
        };

        let fov = FieldOfViewVertices {
            ndc_coo,
            world_coo,
            model_coo,
            great_circles,
        };

        fov
    }

    // Recompute the camera fov vertices when the projection is changing
    pub fn set_projection<P: Projection>(
        &mut self,
        ndc_to_clip: &Vector2<f32>,
        clip_zoom_factor: f32,
        w2m: &Matrix4<f32>,
        aperture: f32,
        longitude_reversed: bool,
    ) {
        self.set_fov::<P>(
            ndc_to_clip,
            clip_zoom_factor,
            w2m,
            aperture,
            longitude_reversed,
        );
    }

    pub fn set_fov<P: Projection>(
        &mut self,
        ndc_to_clip: &Vector2<f32>,
        clip_zoom_factor: f32,
        w2m: &Matrix4<f32>,
        aperture: f32,
        longitude_reversed: bool,
    ) {
        self.world_coo = ndc_to_world::<P>(
            &self.ndc_coo,
            ndc_to_clip,
            clip_zoom_factor,
            longitude_reversed,
        );
        self.set_rotation::<P>(w2m, aperture);
    }

    pub fn set_rotation<P: Projection>(&mut self, w2m: &Matrix4<f32>, aperture: f32) {
        if let Some(world_coo) = &self.world_coo {
            self.model_coo = Some(world_to_model(world_coo, w2m));
        } else {
            self.model_coo = None;
        }

        self.set_great_circles::<P>(aperture);
    }

    fn set_great_circles<P: Projection>(&mut self, aperture: f32) {
        if aperture < P::RASTER_THRESHOLD_ANGLE {
            if let Some(vertices) = &self.model_coo {
                self.great_circles = FieldOfViewType::new_polygon(vertices);
            } else if let FieldOfViewType::Polygon(_) = &self.great_circles {
                self.great_circles = FieldOfViewType::new_allsky();
            }
        } else {
            // We are too unzoomed => we plot the allsky grid
            if let FieldOfViewType::Polygon(_) = &self.great_circles {
                self.great_circles = FieldOfViewType::new_allsky();
            }
        }
    }

    pub fn get_vertices(&self) -> Option<&Vec<ModelCoord>> {
        self.model_coo.as_ref()
    }

    pub fn get_bounding_box(&self) -> &BoundingBox {
        self.great_circles.get_bounding_box()
    }

    pub fn _type(&self) -> &FieldOfViewType {
        &self.great_circles
    }
}
use crate::sphere_geometry::BoundingBox;
use std::iter;

use crate::renderable::projection::Projection;
