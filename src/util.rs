use geo::{geometry::Coord, MultiPoint, MultiPolygon, Point, Polygon, Rect};

pub fn bbox_wgs84_to_rijksdriehoek(bbox: Rect<f64>) -> Rect<f64> {
    use geo::algorithm::map_coords::MapCoords;

    // Note that we invert x and y here
    let project_coord = |p: Coord| {
        let rd = rijksdriehoek::wgs84_to_rijksdriehoek(p.y, p.x);
        Coord { x: rd.0, y: rd.1 }
    };

    bbox.map_coords(&project_coord)
}

/// Merge an iterator of bboxes to a single bbox.
pub fn merge_bbox_iter<I>(iter: I) -> Option<Rect<f64>>
where
    I: Iterator<Item = Rect<f64>>,
{
    fold_first(iter, merge_bboxes)
}

/// Perform a fold over an iterator, where the initial accumulator value is equal to the first
/// iterator value.
///
/// May yield a `None` when the iterator yields no value.
pub fn fold_first<I, X, F>(mut iter: I, func: F) -> Option<X>
where
    I: Iterator<Item = X>,
    F: FnMut(X, X) -> X,
{
    let first = iter.next()?;
    Some(iter.fold(first, func))
}

pub fn polygon_to_bbox(value: geojson::Value) -> Result<Rect<f64>, ()> {
    use geo::algorithm::bounding_rect::BoundingRect;

    let shape: Polygon<f64> = value.try_into().or(Err(()))?;
    shape.bounding_rect().ok_or(())
}

pub fn bbox_to_linestring(bbox: Rect<f64>) -> Result<geojson::Value, ()> {
    let polygon: Polygon<f64> = bbox.try_into().or(Err(()))?;
    Ok(geojson::Value::from(polygon.exterior()))
}

/// Return coordinate with easting (longitude) in x and northing (latitude) in y
pub fn coordinate_rijksdriehoek_to_wgs84(rd_x: f64, rd_y: f64) -> Coord<f64> {
    // Latitude is y and longitude is x
    let (y, x) = rijksdriehoek::rijksdriehoek_to_wgs84(rd_x, rd_y);
    Coord { x, y }
}

/// Merge two bboxes to a single bbox.
pub fn merge_bboxes(acc: Rect<f64>, r: Rect<f64>) -> Rect<f64> {
    Rect::new(
        Coord {
            x: acc.min().x.min(r.min().x),
            y: acc.min().y.min(r.min().y),
        },
        Coord {
            x: acc.max().x.max(r.max().x),
            y: acc.max().y.max(r.max().y),
        },
    )
}

/// Stretch a Rect to a square.
pub fn stretch_to_square(rect: Rect<f64>) -> Rect<f64> {
    use geo::algorithm::centroid::Centroid;

    let (height, width) = (rect.height(), rect.width());
    let (hheight, hwidth) = (height / 2., width / 2.);
    let centroid = rect.centroid();

    if rect.height() < rect.width() {
        rect.min().y = centroid.y() - hwidth;
        rect.max().y = centroid.y() + hwidth;
    } else {
        rect.min().x = centroid.x() - hheight;
        rect.max().x = centroid.x() + hheight;
    }

    rect
}

/// Add a margin to both sides of the Rect.
pub fn add_margin(rect: Rect<f64>, margin: f64) -> Rect<f64> {
    let (min, max): (geo::Point<f64>, geo::Point<f64>) = (rect.min().into(), rect.max().into());
    let margin = geo::Point::new(margin, margin);
    geo::Rect::new(min - margin, max + margin)
}

/// Expand the bounding box to the given size (height and width)
/// Note: only works with rijksdriehoekscoordinates, will probably panic otherwise
pub fn expand_to_size(rect: Rect<f64>, size: f64) -> Rect<f64> {
    // Make sure the rect to a square
    let square_bbox = stretch_to_square(rect);

    // Determine how much margin should be added
    let width = square_bbox.max().y - square_bbox.min().y;
    let margin = (size - width) / 2.0;

    // Return the margin
    add_margin(square_bbox, margin)
}

pub fn points_to_geojson_multipoint(points: Vec<Point<f64>>) -> geojson::GeoJson {
    let mp: MultiPoint<f64> = points.into();
    let geometry = geojson::Geometry::new(geojson::Value::from(&mp));

    geojson::Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: None,
        foreign_members: None,
    }
    .into()
}

pub fn polygons_to_geojson_multipolygon(polygons: Vec<Polygon<f64>>) -> geojson::GeoJson {
    let mp: MultiPolygon<f64> = polygons.into();
    let geometry = geojson::Geometry::new(geojson::Value::from(&mp));

    geojson::Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: None,
        foreign_members: None,
    }
    .into()
}
