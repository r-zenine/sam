use sam_tui::modal_view::{MockValue, ModalView, OptionToggle};

fn main() {
    let mut initial_list = vec![];
    for i in 1..100 {
        initial_list.push(MockValue::new(i, format!("elem {}", i).as_str()));
    }
    let initial_options = vec![
        OptionToggle {
            text: String::from("option"),
            key: 'o',
            active: false,
        },
        OptionToggle {
            text: String::from("not option"),
            key: 'n',
            active: true,
        },
    ];
    let controller = ModalView::new(initial_list, initial_options);
    let response = controller.run();
    println!("Response: {:?}", response);
}
