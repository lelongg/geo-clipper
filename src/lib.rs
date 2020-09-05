//! This crate allows to perform boolean operations on polygons.
//!
//! It makes use of [clipper-sys](https://github.com/lelongg/clipper-sys) which is a binding to the C++ version of [Clipper](http://www.angusj.com/delphi/clipper.php).
//!
//! # Example
//!
//! The following example shows how to compute the intersection of two polygons.  
//! The [`intersection`] method (as well as [`difference`], [`union`] and [`xor`]) is provided by the [`Clipper`] trait which is implemented for some [geo-types](https://docs.rs/geo-types/0.4.3/geo_types/).
//!
//! ```
//! # fn main() {
//! use geo_types::{Coordinate, LineString, Polygon};
//! use geo_clipper::Clipper;
//!
//! let subject = Polygon::new(
//!     LineString(vec![
//!         Coordinate { x: 180.0, y: 200.0 },
//!         Coordinate { x: 260.0, y: 200.0 },
//!         Coordinate { x: 260.0, y: 150.0 },
//!         Coordinate { x: 180.0, y: 150.0 },
//!     ]),
//!     vec![LineString(vec![
//!         Coordinate { x: 215.0, y: 160.0 },
//!         Coordinate { x: 230.0, y: 190.0 },
//!         Coordinate { x: 200.0, y: 190.0 },
//!     ])],
//! );
//!
//! let clip = Polygon::new(
//!     LineString(vec![
//!         Coordinate { x: 190.0, y: 210.0 },
//!         Coordinate { x: 240.0, y: 210.0 },
//!         Coordinate { x: 240.0, y: 130.0 },
//!         Coordinate { x: 190.0, y: 130.0 },
//!     ]),
//!     vec![],
//! );
//!
//! let result = subject.intersection(&clip, 1.0);
//! # }
//! ```
//!
//! [`Clipper`]: trait.Clipper.html
//! [`intersection`]: trait.Clipper.html#method.intersection
//! [`difference`]: trait.Clipper.html#method.difference
//! [`union`]: trait.Clipper.html#method.union
//! [`xor`]: trait.Clipper.html#method.xor

use clipper_sys::{
    execute, execute_offset, free_polygons, ClipType, ClipType_ctDifference,
    ClipType_ctIntersection, ClipType_ctUnion, ClipType_ctXor, EndType as ClipperEndType,
    EndType_etClosedLine, EndType_etClosedPolygon, EndType_etOpenButt, EndType_etOpenRound,
    EndType_etOpenSquare, JoinType as ClipperJoinType, JoinType_jtMiter, JoinType_jtRound,
    JoinType_jtSquare, Path, PolyFillType_pftNonZero, PolyType, PolyType_ptClip,
    PolyType_ptSubject, Polygon as ClipperPolygon, Polygons, Vertice,
};
use geo_types::{Coordinate, LineString, MultiPolygon, Polygon};
use std::convert::TryInto;

#[derive(Clone, Copy)]
pub enum JoinType {
    Square,
    Round,
    Miter,
}

impl From<JoinType> for ClipperJoinType {
    fn from(jt: JoinType) -> Self {
        match jt {
            JoinType::Square => JoinType_jtSquare,
            JoinType::Round => JoinType_jtRound,
            JoinType::Miter => JoinType_jtMiter,
        }
    }
}

impl From<EndType> for ClipperEndType {
    fn from(et: EndType) -> Self {
        match et {
            EndType::ClosedPolygon => EndType_etClosedPolygon,
            EndType::ClosedLine => EndType_etClosedLine,
            EndType::OpenButt => EndType_etOpenButt,
            EndType::OpenSquare => EndType_etOpenSquare,
            EndType::OpenRound => EndType_etOpenRound,
        }
    }
}

pub enum EndType {
    ClosedPolygon,
    ClosedLine,
    OpenButt,
    OpenSquare,
    OpenRound,
}

struct ClipperPolygons {
    pub polygons: Polygons,
    pub factor: f64,
}

struct ClipperPath {
    pub path: Path,
    pub factor: f64,
}

impl From<ClipperPolygons> for MultiPolygon<f64> {
    fn from(polygons: ClipperPolygons) -> Self {
        polygons
            .polygons
            .polygons()
            .iter()
            .filter_map(|polygon| {
                let paths = polygon.paths();
                Some(Polygon::new(
                    ClipperPath {
                        path: *paths.first()?,
                        factor: polygons.factor,
                    }
                    .into(),
                    paths
                        .iter()
                        .skip(1)
                        .map(|path| {
                            ClipperPath {
                                path: *path,
                                factor: polygons.factor,
                            }
                            .into()
                        })
                        .collect(),
                ))
            })
            .collect()
    }
}

impl From<ClipperPath> for LineString<f64> {
    fn from(path: ClipperPath) -> Self {
        path.path
            .vertices()
            .iter()
            .map(|vertice| Coordinate {
                x: vertice[0] as f64 / path.factor,
                y: vertice[1] as f64 / path.factor,
            })
            .collect()
    }
}

#[doc(hidden)]
pub struct OwnedPolygon {
    polygons: Vec<ClipperPolygon>,
    paths: Vec<Vec<Path>>,
    vertices: Vec<Vec<Vec<Vertice>>>,
}

pub trait ToOwnedPolygon {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> OwnedPolygon;
}

impl ToOwnedPolygon for MultiPolygon<f64> {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> OwnedPolygon {
        OwnedPolygon {
            polygons: Vec::with_capacity(self.0.len()),
            paths: Vec::with_capacity(self.0.len()),
            vertices: Vec::with_capacity(self.0.len()),
        }
        .add_polygons(self, poly_type, factor)
    }
}

impl ToOwnedPolygon for Polygon<f64> {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> OwnedPolygon {
        OwnedPolygon {
            polygons: Vec::with_capacity(1),
            paths: Vec::with_capacity(1),
            vertices: Vec::with_capacity(1),
        }
        .add_polygon(self, poly_type, factor)
    }
}

impl OwnedPolygon {
    pub fn get_clipper_polygons(&mut self) -> &Vec<ClipperPolygon> {
        for (polygon, (paths, paths_vertices)) in self
            .polygons
            .iter_mut()
            .zip(self.paths.iter_mut().zip(self.vertices.iter_mut()))
        {
            for (path, vertices) in paths.iter_mut().zip(paths_vertices.iter_mut()) {
                path.vertices = vertices.as_mut_ptr();
                path.vertices_count = vertices.len().try_into().unwrap();
            }

            polygon.paths = paths.as_mut_ptr();
            polygon.paths_count = paths.len().try_into().unwrap();
        }
        &self.polygons
    }

    fn add_polygon(mut self, polygon: &Polygon<f64>, poly_type: PolyType, factor: f64) -> Self {
        let path_count = polygon.interiors().len() + 1;
        self.paths.push(Vec::with_capacity(path_count));
        self.vertices.push(Vec::with_capacity(path_count));
        let last_path = self.paths.last_mut().unwrap();
        let last_path_vertices = self.vertices.last_mut().unwrap();

        for line_string in std::iter::once(polygon.exterior()).chain(polygon.interiors().iter()) {
            last_path_vertices.push(Vec::with_capacity(line_string.0.len().saturating_sub(1)));
            let last_vertices = last_path_vertices.last_mut().unwrap();

            for coordinate in line_string.0.iter().skip(1) {
                last_vertices.push([
                    (coordinate.x * factor) as i64,
                    (coordinate.y * factor) as i64,
                ]);
            }

            last_path.push(Path {
                vertices: std::ptr::null_mut(),
                vertices_count: 0,
                closed: 1,
            });
        }

        self.polygons.push(ClipperPolygon {
            paths: std::ptr::null_mut(),
            paths_count: 0,
            type_: poly_type,
        });

        self
    }

    fn add_polygons(self, polygon: &MultiPolygon<f64>, poly_type: PolyType, factor: f64) -> Self {
        polygon.0.iter().fold(self, |polygons, polygon| {
            polygons.add_polygon(polygon, poly_type, factor)
        })
    }
}

fn execute_offset_operation<T: ToOwnedPolygon + ?Sized>(
    polygons: &T,
    delta: f64,
    jt: JoinType,
    et: EndType,
    factor: f64,
) -> MultiPolygon<f64> {
    let mut owned = polygons.to_polygon_owned(PolyType_ptSubject, factor);
    let mut get_clipper = owned.get_clipper_polygons().clone();
    let clipper_polygons = Polygons {
        polygons: get_clipper.as_mut_ptr(),
        polygons_count: get_clipper.len().try_into().unwrap(),
    };
    let solution = unsafe { execute_offset(clipper_polygons, delta, jt.into(), et.into()) };

    let result = ClipperPolygons {
        polygons: solution,
        factor,
    }
    .into();
    unsafe {
        free_polygons(solution);
    }
    result
}

fn execute_boolean_operation<T: ToOwnedPolygon + ?Sized, U: ToOwnedPolygon + ?Sized>(
    clip_type: ClipType,
    subject_polygons: &T,
    clip_polygons: &U,
    factor: f64,
) -> MultiPolygon<f64> {
    let mut subject_owned = subject_polygons.to_polygon_owned(PolyType_ptSubject, factor);
    let mut clip_owned = clip_polygons.to_polygon_owned(PolyType_ptClip, factor);
    let mut polygons: Vec<ClipperPolygon> = subject_owned
        .get_clipper_polygons()
        .iter()
        .chain(clip_owned.get_clipper_polygons().iter())
        .cloned()
        .collect();
    let clipper_polygons = Polygons {
        polygons: polygons.as_mut_ptr(),
        polygons_count: polygons.len().try_into().unwrap(),
    };

    let solution = unsafe {
        execute(
            clip_type,
            clipper_polygons,
            PolyFillType_pftNonZero,
            PolyFillType_pftNonZero,
        )
    };

    let result = ClipperPolygons {
        polygons: solution,
        factor,
    }
    .into();
    unsafe {
        free_polygons(solution);
    }
    result
}

/// This trait defines the boolean operations between polygons.
///
/// The `factor` parameter in its methods is used to scale shapes before and after applying the boolean operation
/// to avoid precision loss since Clipper (the underlaying library) performs integer computation.
pub trait Clipper<T: ?Sized> {
    fn difference(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn intersection(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn union(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn xor(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
}

pub trait ClipperOffset {
    fn offset(
        &self,
        delta: f64,
        join_type: JoinType,
        end_type: EndType,
        factor: f64,
    ) -> MultiPolygon<f64>;
}

impl<T: ToOwnedPolygon + ?Sized, U: ToOwnedPolygon + ?Sized> Clipper<T> for U {
    fn difference(&self, other: &T, factor: f64) -> MultiPolygon<f64> {
        execute_boolean_operation(ClipType_ctDifference, self, other, factor)
    }

    fn intersection(&self, other: &T, factor: f64) -> MultiPolygon<f64> {
        execute_boolean_operation(ClipType_ctIntersection, self, other, factor)
    }

    fn union(&self, other: &T, factor: f64) -> MultiPolygon<f64> {
        execute_boolean_operation(ClipType_ctUnion, self, other, factor)
    }

    fn xor(&self, other: &T, factor: f64) -> MultiPolygon<f64> {
        execute_boolean_operation(ClipType_ctXor, self, other, factor)
    }
}

impl<T: ToOwnedPolygon + ?Sized> ClipperOffset for T {
    fn offset(
        &self,
        delta: f64,
        join_type: JoinType,
        end_type: EndType,
        factor: f64,
    ) -> MultiPolygon<f64> {
        execute_offset_operation(self, delta, join_type, end_type, factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let expected = MultiPolygon(vec![Polygon::new(
            LineString(vec![
                Coordinate { x: 240.0, y: 200.0 },
                Coordinate { x: 190.0, y: 200.0 },
                Coordinate { x: 190.0, y: 150.0 },
                Coordinate { x: 240.0, y: 150.0 },
            ]),
            vec![LineString(vec![
                Coordinate { x: 200.0, y: 190.0 },
                Coordinate { x: 230.0, y: 190.0 },
                Coordinate { x: 215.0, y: 160.0 },
            ])],
        )]);

        let subject = Polygon::new(
            LineString(vec![
                Coordinate { x: 180.0, y: 200.0 },
                Coordinate { x: 260.0, y: 200.0 },
                Coordinate { x: 260.0, y: 150.0 },
                Coordinate { x: 180.0, y: 150.0 },
            ]),
            vec![LineString(vec![
                Coordinate { x: 215.0, y: 160.0 },
                Coordinate { x: 230.0, y: 190.0 },
                Coordinate { x: 200.0, y: 190.0 },
            ])],
        );

        let clip = Polygon::new(
            LineString(vec![
                Coordinate { x: 190.0, y: 210.0 },
                Coordinate { x: 240.0, y: 210.0 },
                Coordinate { x: 240.0, y: 130.0 },
                Coordinate { x: 190.0, y: 130.0 },
            ]),
            vec![],
        );

        let result = subject.intersection(&clip, 1.0);
        assert_eq!(expected, result);
    }
}
