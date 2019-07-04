use clipper_sys::{
    execute, ClipType, ClipType_ctDifference, ClipType_ctIntersection, ClipType_ctUnion,
    ClipType_ctXor, Path, PolyFillType_pftNonZero, PolyType, PolyType_ptClip, PolyType_ptSubject,
    Polygon as ClipperPolygon, Polygons, Vertice,
};
use geo_types::{Coordinate, LineString, MultiPolygon, Polygon};

pub struct ClipperPolygons {
    pub polygons: Polygons,
    pub factor: f64,
}

pub struct ClipperPath {
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

pub struct PolygonsOwned {
    polygons: Vec<ClipperPolygon>,
    paths: Vec<Vec<Path>>,
    vertices: Vec<Vec<Vec<Vertice>>>,
}

pub trait ToPolygonOwned {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> PolygonsOwned;
}

impl ToPolygonOwned for MultiPolygon<f64> {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> PolygonsOwned {
        PolygonsOwned {
            polygons: Vec::with_capacity(self.0.len()),
            paths: Vec::with_capacity(self.0.len()),
            vertices: Vec::with_capacity(self.0.len()),
        }
        .add_polygons(self, poly_type, factor)
    }
}

impl ToPolygonOwned for Polygon<f64> {
    fn to_polygon_owned(&self, poly_type: PolyType, factor: f64) -> PolygonsOwned {
        PolygonsOwned {
            polygons: Vec::with_capacity(1),
            paths: Vec::with_capacity(1),
            vertices: Vec::with_capacity(1),
        }
        .add_polygon(self, poly_type, factor)
    }
}

impl PolygonsOwned {
    pub fn get_clipper_polygons(&mut self) -> &Vec<ClipperPolygon> {
        for (polygon, (paths, paths_vertices)) in self
            .polygons
            .iter_mut()
            .zip(self.paths.iter_mut().zip(self.vertices.iter_mut()))
        {
            for (path, vertices) in paths.iter_mut().zip(paths_vertices.iter_mut()) {
                path.vertices = vertices.as_mut_ptr();
                path.vertices_count = vertices.len();
            }

            polygon.paths = paths.as_mut_ptr();
            polygon.paths_count = paths.len();
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
            last_path_vertices.push(Vec::with_capacity(line_string.0.len() - 1));
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

pub fn execute_boolean_operation<T: ToPolygonOwned + ?Sized, U: ToPolygonOwned + ?Sized>(
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
        polygons_count: polygons.len(),
    };

    let solution = ClipperPolygons {
        polygons: unsafe {
            execute(
                clip_type,
                clipper_polygons,
                PolyFillType_pftNonZero,
                PolyFillType_pftNonZero,
            )
        },
        factor,
    };

    solution.into()
}

pub trait Clipper<T: ?Sized> {
    fn difference(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn intersection(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn union(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
    fn xor(&self, other: &T, factor: f64) -> MultiPolygon<f64>;
}

impl<T: ToPolygonOwned + ?Sized, U: ToPolygonOwned + ?Sized> Clipper<T> for U {
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
