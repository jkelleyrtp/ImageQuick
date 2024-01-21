use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

use dioxus::{
    desktop::{use_asset_handler, LogicalSize, WindowBuilder},
    router::prelude::*,
};
use dioxus::{
    desktop::{window, wry::http::Response},
    prelude::*,
};
use rgb::{RGB8, RGBA8};

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::default().with_window(
                WindowBuilder::new().with_inner_size(LogicalSize::new(1600.0, 1000.0)),
            ),
        )
        .launch(|| {
            rsx! {
                Router::<Route> {}
            }
        });
}

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Home,

    #[route("/volumes")]
    Volumes,
}

static VOLUMES: GlobalSignal<Vec<PathBuf>> = Signal::global(|| {
    std::fs::read_dir("/Volumes")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            !path
                .iter()
                .last()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Macintosh HD")
        })
        .collect::<Vec<_>>()
});

static VOLUME: GlobalSignal<Option<PathBuf>> = Signal::global(|| VOLUMES.read().first().cloned());

static FILES: GlobalSignal<Files> =
    Signal::global(|| Files::new(VOLUME.read().clone().unwrap().to_str().unwrap().to_string()));

fn Volumes() -> Element {
    rsx! {
        div { }
    }
}

fn Home() -> Element {
    // use a local hook to ensure mut checking
    let mut files = use_hook(|| FILES.signal());

    rsx! {
        style { {include_str!("../src/fileexplorer.css")} }
        h1 { "ImageQuick" }
        h3 { "Reading volume: {VOLUME:?}"}
        link { href:"https://fonts.googleapis.com/icon?family=Material+Icons", rel:"stylesheet" }
        header {
            i { class: "material-icons icon-menu", "menu" }
            h1 { "Files: ", {files.read().current()} }
            span { }
            i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
        }
        div {
            if !files.read().cur_dir_is_image_dir() {
                FileList { }
            } else {
                ImageList { }
            }
        }
        if let Some(err) = files.read().err.as_ref() {
            div {
                code { "{err}" }
                button { onclick: move |_| files.write().clear_err(), "x" }
            }
        }
    }
}

// read the contents of the SD card and generate and cache thumbails
// Ideally if we re-read this card, thumbnails should already exist in our system-level cache
// We should do this by getting the md5 hash without reading the file
// It's not a security issue if the image is out of date, but it will lead to thumbails being wrong
// We should try and bust the cache
// How do we generate a cache? IDK
#[component]
fn ImageList() -> Element {
    // React only to changes in the current directory
    let cur_dir = use_memo(move || FILES.read().current().to_string());
    let files = use_hook(|| FILES.signal());

    // We want to generate and cache thumbnails for all images in the current directory
    use_asset_handler("thumbnails", move |req, res| {
        tokio::task::spawn_blocking(move || {
            // get the image path, stripping the /thumbnails prefix
            let image_path = req
                .uri()
                .path()
                .trim_start_matches("/thumbnails")
                .to_string();

            // check if the image exists in the cache
            let cache_dir = PathBuf::from(
                "/Users/jonkelley/Development/Projects/dioxus-images/cache/thumbnails",
            );

            dbg!(&image_path, &cache_dir);

            let cache_path = cache_dir.join(&image_path.strip_prefix('/').unwrap());

            // if the image exists in the cache, return it
            if cache_path.exists() {
                let image = std::fs::read(cache_path).unwrap();
                return res.respond(Response::new(image));
            }

            // Make sure there's a cache directory
            dbg!(&cache_path);
            std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();

            println!("Loading thumbnail for {image_path}");
            let thumb = create_thumb(image_path.into(), cache_path).unwrap();

            // return the image
            res.respond(Response::new(thumb))
        });
    });

    rsx! {
        div {
            for image in files.read().path_names.iter() {
                img { src: "/thumbnails{image}", class: "thumbnail" }
            }
        }
    }
}

fn create_thumb(
    sd_card_location: PathBuf,
    cache_target_location: PathBuf,
) -> std::io::Result<Vec<u8>> {
    let width;
    let height;
    let colorspace;
    // Create the thumbnail
    let thumb = {
        let mut d =
            mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS).from_path(sd_card_location)?;

        d.scale(1);

        d.width(); // FYI
        d.height();
        // colorspace = d.color_space();

        for marker in d.markers() {
            // dbg!(marker);
        }

        let mut image = d.rgba()?;
        colorspace = image.color_space();
        width = image.width();
        height = image.height();
        dbg!(
            image.color_space(),
            image.width(),
            image.height(),
            colorspace
        );

        let pixels: Vec<RGBA8> = image.read_scanlines()?;
        image.finish()?;
        pixels
    };

    dbg!(width, height, thumb.len(), width * height * 3);

    // And write it to the filesystem
    let mut comp = mozjpeg::Compress::new(colorspace);

    let target = std::fs::File::create(&cache_target_location).unwrap();

    comp.set_size(width, height);
    let mut comp = comp.start_compress(target).unwrap(); // any io::Write will work
                                                         // let mut comp = comp.start_compress(target)?; // any io::Write will work

    // replace with your image data
    let raw = thumb
        .iter()
        .map(|pixel| vec![pixel.r, pixel.g, pixel.b, pixel.a])
        .collect::<Vec<Vec<u8>>>();

    let flattened = raw.iter().flatten().cloned().collect::<Vec<u8>>();

    comp.write_scanlines(&flattened).unwrap();

    let writer = comp.finish().unwrap();

    std::fs::read(&cache_target_location)
}

#[component]
fn Thumbnail() -> Element {
    todo!()
}

#[component]
fn FileList() -> Element {
    rsx! {
        {FILES.read().path_names.iter().enumerate().map(|(dir_id, path)| {
                let path_end = path.split('/').last().unwrap_or(path.as_str());
                rsx! (
                    div { class: "folder", key: "{path}",
                        i { class: "material-icons",
                            onclick: move |_| FILES.write().enter_item(dir_id),
                            if path_end.contains('.') {
                                "description"
                            } else {
                                "folder"
                            }
                            p { class: "cooltip", "0 folders / 0 files" }
                        }
                        h1 { "{path_end}" }
                    }
                )
            })},
    }
}

struct Files {
    path_stack: Vec<String>,
    path_names: Vec<String>,
    err: Option<String>,

    thumbnails: Signal<HashMap<String, String>>,
}

impl Files {
    fn new(start: String) -> Self {
        let mut files = Self {
            path_stack: vec![start],
            path_names: vec![],
            err: None,
            thumbnails: Signal::new(HashMap::new()),
        };

        files.reload_path_list();

        files
    }

    fn reload_path_list(&mut self) {
        let cur_path = self.path_stack.last().unwrap();
        let paths = match std::fs::read_dir(cur_path) {
            Ok(e) => e,
            Err(err) => {
                let err = format!("An error occured: {err:?}");
                self.err = Some(err);
                self.path_stack.pop();
                return;
            }
        };
        let collected = paths.collect::<Vec<_>>();

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in collected {
            self.path_names
                .push(path.unwrap().path().display().to_string());
        }

        // Probe these paths - if we see a moderate amount of jpgs, it's likely a photo directory and we should start preloading thumbnails
        // The smartest approach is to preload the last images first - usually a photographer wants to quickly view the most recent images
        if self.cur_dir_is_image_dir() {
            println!("This is an image directory, we should preload thumbnails!");
        }
    }

    /// Is the current image seemingly a directory with images?
    fn cur_dir_is_image_dir(&self) -> bool {
        let mut jpg_count = 0;
        let mut png_count = 0;

        for path in self.path_names.iter() {
            let path = path.to_ascii_lowercase();
            if path.ends_with(".jpg") {
                jpg_count += 1;
            }

            if path.ends_with(".png") {
                png_count += 1;
            }
        }

        jpg_count > 3 || png_count > 3
    }

    fn go_up(&mut self) {
        if self.path_stack.len() > 1 {
            self.path_stack.pop();
        }
        self.reload_path_list();
    }

    fn enter_item(&mut self, dir_id: usize) {
        let path = &self.path_names[dir_id];
        let as_path = PathBuf::from(path);

        if as_path.is_dir() {
            self.path_stack.push(path.clone());
            self.reload_path_list();
            return;
        }

        if as_path.is_file() {
            if as_path
                .to_path_buf()
                .to_str()
                .unwrap()
                .to_ascii_lowercase()
                .ends_with(".jpg")
            {
                let viewer = VirtualDom::new_with_props(ImageViewer, as_path);

                window().new_window(
                    viewer,
                    dioxus::desktop::Config::default().with_window(
                        WindowBuilder::new().with_inner_size(LogicalSize::new(1600.0, 1000.0)),
                    ).with_custom_head(
                        r#"<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no" />"#.to_string()
                    ),
                );
            }
        }
    }

    fn current(&self) -> &str {
        self.path_stack.last().unwrap()
    }

    fn clear_err(&mut self) {
        self.err = None;
    }
}

fn ImageViewer(path: PathBuf) -> Element {
    rsx! {
        style { {include_str!("./image_viewer.css")} }
        img { src: path.to_str().unwrap(), class: "viewer-image" }
    }
}
