use formulac::Builder;
use num_complex::Complex;
use symb_anafis::diff;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
#[derive(Clone)]
pub struct PointResult {
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl PointResult {
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.x
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.y
    }
}

#[wasm_bindgen]
pub struct Mathematician {
    points_normalised_function: Vec<PointResult>,
    points_normalised_derivative: Vec<PointResult>,
    points_normalised_second_derivative: Vec<PointResult>,
    tangent_function_points: Vec<PointResult>,
    roots: Vec<f64>,
    criticals: Vec<f64>,
    inflections: Vec<f64>,
    function: String,
    derivative: String,
    second_derivative: String,
    start: Option<f64>,
    end: Option<f64>,
    y_offset: f64,
    x0: f64,
}

#[wasm_bindgen]
impl Mathematician {
    #[wasm_bindgen(constructor)]
    pub fn new(function: String) -> Mathematician {
        Mathematician {
            points_normalised_function: Vec::new(),
            points_normalised_derivative: Vec::new(),
            points_normalised_second_derivative: Vec::new(),
            tangent_function_points: Vec::new(),
            roots: Vec::new(),
            criticals: Vec::new(),
            inflections: Vec::new(),
            function,
            derivative: String::new(),
            second_derivative: String::new(),
            start: None,
            end: None,
            y_offset: 0.0,
            x0: 0.0,
        }
    }

    pub fn calc_derivatives(&mut self) -> Result<(), JsValue> {
        self.derivative = compute_derivative(&self.function)?;
        self.second_derivative = compute_derivative(&self.derivative)?;
        Ok(())
    }

    pub fn to_points(&mut self) -> Result<(), JsValue> {
        let start = self
            .start
            .ok_or_else(|| JsValue::from_str("Start range has not been set!"))?;
        let end = self
            .end
            .ok_or_else(|| JsValue::from_str("End range has not been set!"))?;

        let points = compute_given_function(&self.function, start, end)
            .map_err(|err| JsValue::from_str(&err))?;
        self.points_normalised_function = scale_points(&points, start, end, self.y_offset);

        let raw_der = compute_given_function(&self.derivative, start, end)
            .map_err(|err| JsValue::from_str(&err))?;
        self.points_normalised_derivative = scale_points(&raw_der, start, end, self.y_offset);

        let raw_sec = compute_given_function(&self.second_derivative, start, end)
            .map_err(|err| JsValue::from_str(&err))?;
        self.points_normalised_second_derivative =
            scale_points(&raw_sec, start, end, self.y_offset);

		let _ = self.calc_roots_critials_inflections();
		let _ = self.tangent_to_function();

        Ok(())
    }

    pub fn add_offset(&mut self, x_shift: f64, y_shift: f64) -> Result<(), JsValue> {
        if let Some(s) = self.start.as_mut() {
            *s += x_shift;
        } else {
            return Err(JsValue::from_str("Start range not set!"));
        }

        if let Some(e) = self.end.as_mut() {
            *e += x_shift;
        } else {
            return Err(JsValue::from_str("End ranges not set!"));
        }

        self.y_offset += y_shift;
        self.calc_derivatives()?;
        self.to_points()?;
        Ok(())
    }

    pub fn zoom(&mut self, factor: f64) -> Result<(), JsValue> {
        let start = self
            .start
            .ok_or_else(|| JsValue::from_str("Start range not set!"))?;
        let end = self
            .end
            .ok_or_else(|| JsValue::from_str("End range not set!"))?;

        let center = (start + end) / 2.0;
        let half_width = (end - start) * factor / 2.0;

        self.start = Some(center - half_width);
        self.end = Some(center + half_width);

        self.calc_derivatives()?;
        self.to_points()?;
        Ok(())
    }

    //only tangent to the function not the derivatives
    pub fn tangent_to_function(&mut self) -> Result<(), JsValue> {
        //y = f'(x_0)(x-x_0) + f(x_0)
        let start = self
            .start
            .ok_or_else(|| JsValue::from_str("Start range not set!"))?;
        let end = self
            .end
            .ok_or_else(|| JsValue::from_str("End range not set!"))?;

        let builder_d = Builder::new(&self.derivative, ["x"]);
        let func_d = builder_d.compile().expect("Failed to compile derivative.");
        let slope_result = func_d([Complex::new(self.x0, 0.0)]);
        let slope = if slope_result.re.is_finite() {
            slope_result.re
        } else {
            0.0
        };

        let builder_f = Builder::new(&self.function, ["x"]);
        let func = builder_f.compile().expect("Failed to compile.");
        let y_result = func([Complex::new(self.x0, 0.0)]);
        let y0 = if y_result.re.is_finite() {
            y_result.re
        } else {
            0.0
        };

        let mut current = start;
        let mut raw_tangent_points = Vec::new();
        let step: f64 = (end - start) / 1000.0;

        for _ in 0..1000 {
            let y_val = slope * (current - self.x0) + y0;
            raw_tangent_points.push((current, y_val));
            current += step;
        }

        self.tangent_function_points = scale_points(&raw_tangent_points, start, end, self.y_offset);

        Ok(())
    }

    pub fn calc_roots_critials_inflections(&mut self) -> Result<(), JsValue> {
        self.roots.clear();
        self.criticals.clear();
        self.inflections.clear();

        let pts_f = &self.points_normalised_function;
        let pts_c = &self.points_normalised_derivative;
        let pts_i = &self.points_normalised_second_derivative;

        for i in 0..1000 {
            let p1_f = &pts_f[i];
            let p1_c = &pts_c[i];
            let p1_i = &pts_i[i];

            if p1_f.y.abs() < f64::EPSILON {
                self.roots.push(p1_f.x);
            }

            if p1_c.y.abs() < f64::EPSILON {
                self.criticals.push(p1_c.x);
            }

            if p1_i.y.abs() < f64::EPSILON {
                self.inflections.push(p1_i.x);
            }
        }
        Ok(())
    }

    //======================================
    //getters 
    #[wasm_bindgen(getter)]
    pub fn function(&self) -> String {
        self.function.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn derivative(&self) -> String {
        self.derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn second_derivative(&self) -> String {
        self.second_derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn points_normalised_function(&self) -> Vec<PointResult> {
        self.points_normalised_function.clone()
    }


    #[wasm_bindgen(getter)]
	pub fn roots(&self) -> Vec<f64> {
		self.roots.clone()
	}

    #[wasm_bindgen(getter)]
	pub fn criticals(&self) -> Vec<f64> {
		self.criticals.clone()
	}

    #[wasm_bindgen(getter)]
	pub fn inflections(&self) -> Vec<f64> {
		self.inflections.clone()
	}

    #[wasm_bindgen(getter)]
    pub fn points_normalised_derivative(&self) -> Vec<PointResult> {
        self.points_normalised_derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn points_normalised_second_derivative(&self) -> Vec<PointResult> {
        self.points_normalised_second_derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn tangent_function_points(&self) -> Vec<PointResult> {
        self.tangent_function_points.clone()
    }
	
	//=====================
	//Setters
    #[wasm_bindgen(setter)]
    pub fn set_start(&mut self, start: f64) {
        self.start = Some(start);
    }

    #[wasm_bindgen(setter)]
    pub fn set_end(&mut self, end: f64) {
        self.end = Some(end);
    }

    #[wasm_bindgen(setter)]
    pub fn set_x0(&mut self, x0: f64) {
        self.x0 = x0;
    }
}

pub fn compute_given_function(
    expr_str: &str,
    start: f64,
    end: f64,
) -> Result<Vec<(f64, f64)>, String> {
    let builder = Builder::new(expr_str, ["x"]);
    let func = builder.compile().expect("Failed to compile: {expr_str}");

    let mut current = start;
    let mut points = Vec::new();
    let step: f64 = (end - start) / 1000.0;

    for _ in 0..1000 {
        let result = func([Complex::new(current, 0.0)]);
        // If the result is NaN or Infinity, fallback to 0.0 (or handle as needed)
        // so the array size remains strictly 1000.
        let y_val = if result.re.is_finite() {
            result.re
        } else {
            0.0
        };

        points.push((current, y_val));
        current += step;
    }

    // while current <= end + 1e-9 {
    //     let result = func([Complex::new(current, 0.0)]);
    //     points.push((current, result.re));
    //     current += step;
    // }

    Ok(points)
}

pub fn scale_points(
    points: &Vec<(f64, f64)>,
    start: f64,
    end: f64,
    y_offset: f64,
) -> Vec<PointResult> {
    if points.is_empty() {
        return Vec::new();
    }

    let scale_mult_x: f64 = 2.0 / (end - start);
    let scale_offset_x: f64 = -1.0 - (scale_mult_x * start);

    // let mut min_y = points[0].1;
    // let mut max_y = min_y;

    // for &(_x, y) in points {
    // if y < min_y {
    // min_y = y;
    // } else if y > max_y {
    // max_y = y;
    // }
    // }
    // let y_range = max_y - min_y;

    // let (scale_mult_y, scale_offset_y) = if y_range.abs() < f64::EPSILON {
    // (0.0, 0.0)
    // } else {
    // let mult = 2.0 / y_range;
    // (mult, -1.0 - (mult * min_y))
    // };

    let min_y = -10.0;
    let max_y = 10.0;
    let y_range = max_y - min_y;
    let mult = 2.0 / y_range;
    let scale_offset_y = -1.0 - (mult * min_y);

    points
        .iter()
        .map(|&(x, y)| {
            let scaled_x = (x * scale_mult_x) + scale_offset_x;
            // let scaled_y = if y_range.abs() < f64::EPSILON {
            // 0.0 + y_offset
            // } else {
            // (y * scale_mult_y) + scale_offset_y + y_offset
            // };
            let scaled_y = (y * mult) + scale_offset_y + y_offset;

            PointResult {
                x: scaled_x,
                y: scaled_y,
            }
        })
        .collect()
}

pub fn points_to_js_array(points: Vec<PointResult>) -> js_sys::Array {
    let js_array = js_sys::Array::new();
    for pt in points {
        let js_val = JsValue::from(pt);
        js_array.push(&js_val);
    }
    js_array
}

#[wasm_bindgen]
pub fn evaluate_function(
    expr_str: &str,
    start: f64,
    end: f64,
    y_offset: f64,
) -> Result<js_sys::Array, JsValue> {
    let points =
        compute_given_function(expr_str, start, end).map_err(|err| JsValue::from_str(&err))?;
    let scaled_points = scale_points(&points, start, end, y_offset);
    let js_array = points_to_js_array(scaled_points);

    Ok(js_array)
}

pub fn compute_derivative(expr_str: &str) -> Result<String, JsValue> {
    let derivative = diff(expr_str.to_string(), "x".to_string(), Some(&[]), None)
        .map_err(|err| JsValue::from_str(&format!("Derivation error: {:?}", err)))?;

    Ok(derivative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_offset_changes_points_and_range() {
        let mut mathematician = Mathematician::new("x^2".to_string());
        mathematician.set_start(-10.0);
        mathematician.set_end(10.0);

        mathematician.calc_derivatives().unwrap();
        mathematician.to_points().unwrap();

        let before_points = mathematician.points_normalised_function();
        let before_start = mathematician.start;
        let before_end = mathematician.end;
        let before_y_offset = mathematician.y_offset;

        mathematician.add_offset(2.0, 1.5).unwrap();

        let after_points = mathematician.points_normalised_function();

        assert_ne!(before_y_offset, mathematician.y_offset);
        assert_ne!(before_start, mathematician.start);
        assert_ne!(before_end, mathematician.end);

        let x_before = before_points.first().map(|p| p.x()).unwrap_or_default();
        let y_before = before_points.first().map(|p| p.y()).unwrap_or_default();
        let x_after = after_points.first().map(|p| p.x()).unwrap_or_default();
        let y_after = after_points.first().map(|p| p.y()).unwrap_or_default();

        assert_ne!(x_before, x_after);
        assert_ne!(y_before, y_after);
    }

    #[test]
    fn zoom_changes_points_and_range() {
        let mut mathematician = Mathematician::new("x^2".to_string());
        mathematician.set_start(-10.0);
        mathematician.set_end(10.0);

        mathematician.calc_derivatives().unwrap();
        mathematician.to_points().unwrap();

        let before_points = mathematician.points_normalised_function();
        let before_start = mathematician.start;
        let before_end = mathematician.end;

        mathematician.zoom(0.5).unwrap();

        let after_points = mathematician.points_normalised_function();

        assert_ne!(before_start, mathematician.start);
        assert_ne!(before_end, mathematician.end);

        let x_before = before_points.first().map(|p| p.x()).unwrap_or_default();
        let y_before = before_points.first().map(|p| p.y()).unwrap_or_default();
        let x_after = after_points.first().map(|p| p.x()).unwrap_or_default();
        let y_after = after_points.first().map(|p| p.y()).unwrap_or_default();

        assert_ne!(x_before, x_after);
        assert_ne!(y_before, y_after);
    }
}
