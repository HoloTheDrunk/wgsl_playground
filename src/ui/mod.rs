mod element;
mod shapes;

use element::Element;

use std::fmt::Debug;

use {
    glam::Vec2,
    indoc::{formatdoc, indoc},
    wgpu::Color,
};

pub mod prelude {
    pub use super::{element::*, shapes::*, SdfObject, Ui, UiTheme, UiThemeBorders, UiThemeColors};
}

pub trait SdfObject: Debug {
    fn dist(&self, pos: Vec2) -> f32;
    fn fn_call(&self) -> String;
}

pub struct UiThemeColors {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
}

pub struct UiThemeBorders {
    pub enabled: bool,
    pub offset: f32,
    pub width: f32,
}

pub struct UiTheme {
    pub colors: UiThemeColors,
    pub borders: UiThemeBorders,
}

pub struct Ui {
    pub theme: UiTheme,
    pub tree: Element,
}

impl Ui {
    pub fn wgsl_shader(&self) -> String {
        let name = "ui";
        let function = self.tree.to_wgsl_function(name);

        let border_consts = if self.theme.borders.enabled {
            let UiThemeBorders { offset, width, .. } = self.theme.borders;
            formatdoc! {
                "const BORDER_OFFSET: f32 = {offset:?};
                const BORDER_WIDTH: f32 = {width:?};"
            }
        } else {
            "".to_owned()
        };

        let ret = if self.theme.borders.enabled {
            let Color { r, g, b, a } = self.theme.colors.primary;
            // Extra indent for the final composition to look nice
            formatdoc! {"
                return select(
                        color,
                        vec4f({r}, {g}, {b}, {a}),
                        abs(dist - BORDER_OFFSET) < BORDER_WIDTH
                    );"
            }
        } else {
            "return color;".to_owned()
        };

        let Color { r, g, b, a } = self.theme.colors.secondary;
        formatdoc! {r#"
            // Vertex shader
            //% include "lib/utils/gen_triangle_vs"

            // Fragment shader
            //% include "lib/sdf"

            {function}

            {border_consts}

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
                let uv = vec2f(in.tex_coords.x, in.tex_coords.y);
                let dist = {name}(uv);
                let color = vec4f({r}, {g}, {b}, {a} * f32(dist < 0.));
                {ret}
            }}
        "#}
    }
}
