extern crate base64;
use std::collections::HashMap;
use std::io::Cursor;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use gloo::dialogs::alert;
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use image::{ImageBuffer, ImageFormat, ImageOutputFormat, Pixel, RgbaImage};
use rand::Rng;
use web_sys::{DragEvent, Event, FileList, HtmlInputElement};
use yew::html::TargetCast;
use yew::{html, Callback, Component, Context, Html};

struct FileDetailPairs {
    name: String,
    file_type: String,
    image_rgba: RgbaImage,
    data: Vec<u8>,
    new_data: Vec<u8>,
}

pub enum ConfigType {
    ColQu(u8),
    PixSw(u8),
}

pub enum Msg {
    Loaded(String, String, Vec<u8>),
    Files(Vec<File>),
    UpdateConfig(ConfigType),
}

pub struct App {
    readers: HashMap<String, FileReader>,
    image_pairs: Vec<FileDetailPairs>,
    color_quantization: u8,
    pixel_switch: u8,
}

pub fn calc_new_image(ori: &RgbaImage, color_quantization: u8, pixel_switch: u8) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let (w, h) = (ori.width(), ori.height());
    let new_image: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        let (x, y) = if x ^ 1 < w && y ^ 1 < h && pixel_switch as usize > rng.gen::<usize>() % 10 {
            (x ^ 1, y ^ 1)
        } else {
            (x, y)
        };
        let px = ori.get_pixel(x, y);
        px.map(|v| {
            if v.leading_ones() as u8 > 8 - color_quantization  {
                255 // 避免得出256溢出
            } else {
                ((v >> color_quantization) + (v >> (color_quantization - 1) & 1))
                    << color_quantization
            }
        })
    });
    let mut buff = Cursor::new(Vec::new());
    new_image
        .write_to(&mut buff, ImageOutputFormat::Png)
        .unwrap();
    buff.into_inner()
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            readers: HashMap::default(),
            image_pairs: Vec::default(),
            // 以2^5为单位
            color_quantization: 5,
            // 30% 概率对角交换像素
            pixel_switch: 3,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Loaded(file_name, file_type, data) => {
                self.readers.remove(&file_name);

                if file_type.contains("image") {
                    let image = image::io::Reader::with_format(
                        Cursor::new(data.clone()),
                        ImageFormat::from_mime_type(&file_type).unwrap(),
                    )
                    .decode()
                    .unwrap();

                    let image_rgba = image.into_rgba8();
                    let new_data =
                        calc_new_image(&image_rgba, self.color_quantization, self.pixel_switch);

                    self.image_pairs.push(FileDetailPairs {
                        name: file_name,
                        file_type,
                        image_rgba,
                        data,
                        new_data,
                    });
                    true
                } else {
                    alert("不是图片");
                    false
                }
            }
            Msg::Files(files) => {
                for file in files.into_iter() {
                    let file_name = file.name();
                    let file_type = file.raw_mime_type();

                    let task = {
                        let link = ctx.link().clone();
                        let file_name = file_name.clone();

                        gloo::file::callbacks::read_as_bytes(&file, move |res| {
                            link.send_message(Msg::Loaded(
                                file_name,
                                file_type,
                                res.expect("failed to read file"),
                            ))
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                true
            }
            Msg::UpdateConfig(conf) => {
                match conf {
                    ConfigType::ColQu(x) => self.color_quantization = x,
                    ConfigType::PixSw(x) => self.pixel_switch = x,
                }
                self.image_pairs.iter_mut().for_each(|ip| {
                    ip.new_data =
                        calc_new_image(&ip.image_rgba, self.color_quantization, self.pixel_switch)
                });
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="wrapper">
                <p id="title">{ "去除暗水印" }</p>
                <label for="file-upload">
                    <div
                        id="drop-container"
                        ondrop={ctx.link().callback(|event: DragEvent| {
                            event.prevent_default();
                            let files = event.data_transfer().unwrap().files();
                            Self::upload_files(files)
                        })}
                        ondragover={Callback::from(|event: DragEvent| {
                            event.prevent_default();
                        })}
                        ondragenter={Callback::from(|event: DragEvent| {
                            event.prevent_default();
                        })}
                    >
                        <p>
                            {"拖拽图片到此处或者点击选择图片"}<br/>
                            {"(可多选)"}
                        </p>
                    </div>
                </label>
                <input
                    id="file-upload"
                    type="file"
                    accept="image/*"
                    multiple={true}
                    onchange={ctx.link().callback(move |e: Event| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        Self::upload_files(input.files())
                    })}
                />
                <div class="range-box">
                    <label>{"颜色量化:"}</label>
                    <input
                        id="color-quan-range"
                        type="range"
                        min="1"
                        max="7"
                        value={self.color_quantization.to_string()}
                        onchange={ctx.link().callback(move |e: Event| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            Msg::UpdateConfig(ConfigType::ColQu(input.value().parse().unwrap()))
                        })}
                    />
                </div>
                <div class="range-box">
                    <label>{"像素交换:"}</label>
                    <input
                        id="color-quan-range"
                        type="range"
                        min="0"
                        max="10"
                        value={self.pixel_switch.to_string()}
                        onchange={ctx.link().callback(move |e: Event| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            Msg::UpdateConfig(ConfigType::PixSw(input.value().parse().unwrap()))
                        })}
                    />
                </div>
                <div id="preview-area">
                    { for self.image_pairs.iter().rev().map(Self::view_file) }
                </div>
            </div>
        }
    }
}

impl App {
    fn view_file(ip: &FileDetailPairs) -> Html {
        html! {
            <div class="preview-box">
                <p class="preview-name">{ &ip.name }</p>
                <div class="preview-medias">
                    <div class="preview-media">
                        <img src={format!("data:{};base64,{}", ip.file_type, STANDARD.encode(&ip.data))} />
                    </div>
                    <div class="preview-media">
                        <img src={format!("data:{};base64,{}", "image/png", STANDARD.encode(&ip.new_data))} />
                    </div>
                </div>
            </div>
        }
    }

    fn upload_files(files: Option<FileList>) -> Msg {
        let mut result = Vec::new();

        if let Some(files) = files {
            let files = js_sys::try_iter(&files)
                .unwrap()
                .unwrap()
                .map(|v| web_sys::File::from(v.unwrap()))
                .map(File::from);
            result.extend(files);
        }
        Msg::Files(result)
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
