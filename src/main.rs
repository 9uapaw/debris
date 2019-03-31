extern crate debris;
use debris::field::Destination;
use debris::field::DestinationLocation;
use debris::field::ElementSelection;
use debris::field::FieldIdentity;
use debris::path::PathBuilder;
use debris::path::PathStep;
use debris::populator::MultiplePopulator;
use debris::populator::SearchDetail;
use debris::populator::SinglePopulator;

fn main() {
    let html = r#"<div class=found>
    <i>HELLO</i>
    <i>WORLD</i>
    <i>!</i>
    </div>"#;
    let path = PathBuilder::new()
        .start(Destination::new(
            r#"div[class="found"]"#,
            ElementSelection::first(),
        ))
        .find_all("i", "", DestinationLocation::Text)
        .build();
    let mut search = SearchDetail::new();
    search.insert_field(
        "test",
        r#"div[class="found"]"#,
        DestinationLocation::Attr(String::from("class")),
        ElementSelection::first(),
    );
    search.insert_path(path);

    let mut populator = SinglePopulator::new(&html, search);
    populator.populate();
    for (k, v) in populator.map {
        println!("KEY: {} - VALUE: {}", k, v);
    }

    for v in populator.values {
        print!("{} ", v);
    }
    //    let mut first_path = Vec::new();
    //    let mut field_map = HashMap::new();
    //    field_map.insert(
    //        String::from("postalCode"),
    //        FieldIdentity {
    //            destination: Destination(r#"span"#.to_string(), ElementSelection::Single(0)),
    //            destination_location: DestinationLocation::Text,
    //        },
    //    );
    //    first_path.push(PathStep::Start(Destination(
    //        String::from("i[class=address]"),
    //        ElementSelection::Single(0),
    //    )));
    //    first_path.push(PathStep::Populate(field_map));
    //    let mut path = PathBuilder::new()
    //        .start(Destination::new("#results", ElementSelection::first()))
    //        .find_all(
    //            ".title",
    //            "",
    //            DestinationLocation::Attr(String::from("href")),
    //        )
    //        .build();
    //    let mut search_detail = Box::new(SearchDetail::new());
    //    search_detail.insert_path(first_path);
    //
    //    let mut multi_pop = MultiplePopulator::new("https://port.hu/programkereso/fesztival?q=&interval=today&events_from=2019-03-21&events_until=2019-03-22&dft=i&cityMain=1&city=cityList-3372&area=festival&ageLimitFrom=2&ageLimitTo=10&s=start&onlyFav=0&documentId=&event-location=1&event-date=2019-03-21",
    //    path, Some(&|link| {
    //            String::from("https://port.hu") + &link
    //        }), *search_detail);
    //
    //    //    multi_pop.populate();
    //    multi_pop.par_populate();
    //
    //    for links in multi_pop.populated_links {
    //        println!("----");
    //        for (field, value) in links {
    //            println!("{} : {}", field, value);
    //        }
    //        println!("----");
    //    }
}
