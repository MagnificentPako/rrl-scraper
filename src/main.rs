#[macro_use]
extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate select;
extern crate wkhtmltopdf;

use clap::App;
use wkhtmltopdf::*;
use regex::Regex;
use select::document::Document;
use select::predicate::{And, Attr, Class, Name, Predicate};
use std::io::Read;

struct Metadata {
    title: String,
    author: String,
}

fn get_metadata(fiction: String) -> Metadata {
    let mut resp = reqwest::get(&format!("http://royalroadl.com/fiction/{}", fiction)).unwrap();
    let mut content = String::new();
    resp.read_to_string(&mut content).unwrap();

    let doc = Document::from(content.as_str());
    let title = doc.find(Class("fic-title").descendant(Name("h1")))
        .next()
        .unwrap();
    let author = doc.find(
        And(Class("col-md-5"), Name("div"))
            .descendant(Name("h4"))
            .descendant(Name("span"))
            .descendant(Name("a")),
    ).next()
        .unwrap();

    Metadata {
        title: title.text(),
        author: author.text(),
    }
}

fn chapters_from_fiction(fiction: String) -> Vec<String> {
    let mut resp = reqwest::get(&format!("http://royalroadl.com/fiction/{}", fiction)).unwrap();
    let mut content = String::new();
    resp.read_to_string(&mut content).unwrap();

    let doc = Document::from(content.as_str());
    let chapters = doc.find(
        Name("table")
            .descendant(Name("tbody"))
            .descendant(Name("tr")),
    );

    let re = Regex::new("/chapter/(.+)/.+$").unwrap();
    let mut output = Vec::<String>::new();

    for chapter in chapters {
        let cap = re.captures(chapter.attr("data-url").unwrap())
            .unwrap()
            .get(1)
            .unwrap();
        output.push(cap.as_str().into());
    }

    output
}

fn chapter_to_html(chapter: &String) -> String {
    let url = &format!(
        "http://royalroadl.com/fiction/chapter/{}",
        chapter);
    let mut respp = reqwest::get(url);
    while respp.is_err() {
        respp = reqwest::get(url);
    }
    
    let mut resp = respp.unwrap();

    let mut content = String::new();
    resp.read_to_string(&mut content).unwrap();

    let doc = Document::from(content.as_str());
    let title = doc.find(Name("a").descendant(Name("h2"))).next().unwrap();
    let author = doc.find(And(Name("div"), Class("col-md-5")).descendant(Name("h3")))
        .next()
        .unwrap();
    let chapter_title = doc.find(And(Name("div"), Class("col-md-5")).descendant(Name("h1")))
        .next()
        .unwrap();
    println!("Downloading chapter [{}]", chapter_title.text());

    let chapter_content_node = doc.find(And(Class("chapter-inner"), Class("chapter-content")))
        .next()
        .unwrap();

    let mut chapter_content = Vec::new();
    for child in chapter_content_node.children() {
        if child.text().trim().len() > 0 {
            chapter_content.push(child.text());
        }
    }

    format!(
        "{}{}",
        //title.html(),
        //author.html(),
        chapter_title.html(),
        chapter_content_node.html()
    )
}
//
//
fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let subname = matches.subcommand_name().unwrap_or("nothing");

    match matches.subcommand() {
        ("fiction", Some(sub_m)) => {
            let fiction: String = sub_m.value_of("ID").unwrap().into();
            let metadata = get_metadata(fiction.clone());
            println!("Starting download of [{}]", metadata.title);
            let chapters = chapters_from_fiction(fiction);
            let mut pdf_app = PdfApplication::new().expect("failed to create pdf app");
            let mut pdfout = pdf_app
                .builder()
                .orientation(Orientation::Portrait)
                .margin(Size::Millimeters(16))
                .title(&metadata.title)
                .build_from_html(format!(
                    "<html><head><meta charset=\"utf-8\"></head><body>{}{}{}</body></html>",
                    format!("<h1>{}</h1>", metadata.title),
                    format!("</br><h2>{}</h2>", metadata.author),
                    chapters
                        .iter()
                        .map(|c| chapter_to_html(c))
                        .fold(String::new(), |acc, c| format!("{}{}", acc, c))
                ))
                .expect("failed to build pdf");

            pdfout.save("out.pdf").expect("failed to save pdf");
        }
        ("chapter", Some(sub_m)) => {
            let chapter: String = sub_m.value_of("ID").unwrap().into();
            println!("Starting download");
            let chapter_html = chapter_to_html(&chapter);
            let mut pdf_app = PdfApplication::new().expect("failed to create pdf app");
            let mut pdfout = pdf_app
                .builder()
                .orientation(Orientation::Portrait)
                .margin(Size::Millimeters(16))
                .title("")
                .build_from_html(format!(
                    "<html><head><meta charset=\"utf-8\"></head><body>{}</body></html>",
                    chapter_html
                ))
                .expect("failed to build pdf");

            pdfout.save("out.pdf").expect("failed to save pdf");
        }
        _ => {}
    }
}
