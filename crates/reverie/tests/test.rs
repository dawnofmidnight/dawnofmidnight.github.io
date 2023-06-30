macro_rules! tests {
    ($($($name:ident => $source:literal),+ $(,)?)?) => {
        $($(
            #[test]
            fn $name() {
                let output = ::reverie::to_html($source).expect("failed to parse");
                let output = ::std::string::String::from_utf8(output).unwrap();
                insta::with_settings!({
                    description => $source,
                    omit_expression => true,
                }, {
                    insta::assert_snapshot!(output);
                });
            }
        )+)?
    };
}

tests! {
    simple => "hello world",
}
