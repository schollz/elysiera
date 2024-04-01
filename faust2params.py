import re, subprocess

# -- Generate dsp.rs

cmd = "faust -lang rust ./faust/dsp.dsp -o ./src/dsp.rs"
subprocess.run(cmd, shell=True)

# add "mod dsp {" to the beginning of the file
# add "}" to the end of the file

pre = """\
mod dsp {
#![allow(clippy::all)]
#![allow(unused_parens)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(non_upper_case_globals)]

use faust_types::*;
"""

with open("src/dsp.rs", "r") as file:
    content = file.read()
    content = content.replace("F32", "f32")
    content = pre + content + "}\npub use dsp::mydsp as ElysieraDSP;"
with open("src/dsp.rs", "w") as file:
    file.write(content)

print("Generated dsp.rs")

# -- Generate params.rs

params = []
with open("src/dsp.rs", "r") as file:
    lines = file.readlines()
    for line in lines:
        if "_slider" in line:
            name = re.search(r'\"(.*?)\"', line).group(1)
            index = re.search(r'ParamIndex\((.*?)\)', line).group(1)
            params.append((name, index))

print(params)

# Convert snake case to Rust's camel case
def case(a):
    return a[0] + "".join([word.capitalize() for word in name.split("_")])[1:]

def float_param(id):
    return f"""\
    #[id = "{id}"]
    pub {id}: FloatParam,
"""

def search_name(line):
    return re.search(r'(?:\[\d])([a-zA-Z0-9_ ]*)', line).group(1).strip()

path = "faust/dsp.dsp"
content = """\
// Generated by scripts/build_param.py
use std::sync::Arc;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use faust_types::{FaustDsp, ParamIndex};

use crate::editor;
\n"""
param_fn = """
fn param_st(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {min, max},
    )
    .with_unit(" st")
    .with_smoother(SmoothingStyle::Linear(50.0))
    .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn param_q(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-1.0),
        },
    )
    .with_smoother(SmoothingStyle::Logarithmic(100.0))
    .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn param_hz(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Skewed {
            min,
            max,
            factor: FloatRange::skew_factor(-1.0),
        },
    )
    .with_smoother(SmoothingStyle::Logarithmic(100.0))
    .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
    .with_string_to_value(formatters::s2v_f32_hz_then_khz())
}

fn param_float(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {min, max},
    )
    .with_smoother(SmoothingStyle::Linear(50.0))
    .with_value_to_string(formatters::v2s_f32_rounded(2))
}

fn param_percentage(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {min, max},
    )
    .with_unit(" %")
    .with_smoother(SmoothingStyle::Linear(50.0))
    .with_value_to_string(formatters::v2s_f32_percentage(2))
    .with_string_to_value(formatters::s2v_f32_percentage())
}

fn param_s(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {min, max},
    )
    .with_unit(" s")
    .with_smoother(SmoothingStyle::Linear(50.0))
    .with_value_to_string(formatters::v2s_f32_rounded(3))
}

fn param_ms(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {min, max},
    )
    .with_unit(" ms")
    .with_smoother(SmoothingStyle::Linear(50.0))
    .with_value_to_string(formatters::v2s_f32_rounded(3))
}
"""
content += """#[derive(Params)]
pub struct ElysieraParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
\n"""
with open(path, "r") as file:
    lines = file.readlines()
    for line in lines:
        if "slider" in line:
            name = search_name(line)\
                .lower()\
                .replace(' ', '_')
            content += float_param(name)
    content += "}\n\nimpl Default for ElysieraParams {\n"
    content += "    fn default() -> Self {\n"
    content += "        Self {\n"
    content += "            editor_state: editor::default_state(),\n\n"
    for line in lines:
        if "slider" in line:
            name = search_name(line)
            name_case = name.lower().replace(' ', '_')

            val = line.split(",")
            default = val[1].strip()
            min = val[2].strip()
            max = val[3].split(")")[0].strip()

            # content += f"            {name}: param(\"{name}\", {default}, {min}, {max}),\n"
            content += f"            {name_case}: param_float(\"{name}\", {default}, {min}, {max}),\n"

    content += "        }\n"
    content += "    }\n"
    content += "}\n\n"

content += f"impl ElysieraParams {{\n"
content += f"    pub fn dsp_set_params(&self, dsp: &mut Box<crate::dsp::ElysieraDSP>) {{\n"
for name, index in params:
    name = name\
        .replace(' ', '_')\
        .upper()
    content += f"        dsp.set_param({name.upper().replace(' ', '_')}, self.{name.lower()}.value());\n"
content += "    }\n"
content += "}\n\n"

for name, index in params:
    name = name\
        .replace(' ', '_')\
        .upper()
    content += f"pub const {name.upper().replace(' ', '_')}: ParamIndex = ParamIndex({index});\n"

content += param_fn

with open("src/params.rs", "w") as file:
    file.write(content)

print("Generated params.rs")