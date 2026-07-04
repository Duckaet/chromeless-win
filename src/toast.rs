pub fn show_toast_js(msg: &str) -> String {
    let escaped = msg
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!(
        r#"
(function(){{
var e=document.getElementById('chromeless-toast');
if(e)e.remove();
var t=document.createElement('div');
t.id='chromeless-toast';
t.textContent='{}';
t.style.cssText='position:fixed;bottom:28px;left:50%;transform:translateX(-50%);padding:8px 20px;background:rgba(28,28,32,0.92);-webkit-backdrop-filter:blur(16px)saturate(1.4);backdrop-filter:blur(16px)saturate(1.4);color:#e8e8ee;border-radius:17px;font:13px/1.4 -apple-system,BlinkMacSystemFont,sans-serif;font-weight:500;z-index:2147483647;pointer-events:none;box-shadow:0 4px 20px rgba(0,0,0,0.3);transition:opacity .4s ease-out;opacity:1;border:1px solid rgba(255,255,255,0.06);white-space:nowrap';
document.body.appendChild(t);
setTimeout(function(){{t.style.opacity='0';setTimeout(function(){{if(t.parentNode)t.remove()}},400)}},1300);
}})();
"#,
        escaped
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_self_executing_function() {
        let js = show_toast_js("hello");
        assert!(js.contains("(function(){"));
        assert!(js.contains("})()"));
    }

    #[test]
    fn output_creates_toast_div() {
        let js = show_toast_js("test");
        assert!(js.contains("document.createElement('div')"));
        assert!(js.contains("t.id='chromeless-toast'"));
    }

    #[test]
    fn output_sets_text_content() {
        let js = show_toast_js("hello world");
        assert!(js.contains("t.textContent='hello world'"));
    }

    #[test]
    fn plain_message_no_escaping_needed() {
        let js = show_toast_js("simple message");
        assert!(js.contains("t.textContent='simple message'"));
    }

    #[test]
    fn empty_string_message() {
        let js = show_toast_js("");
        assert!(js.contains("t.textContent=''"));
    }

    #[test]
    fn backslash_is_escaped() {
        let js = show_toast_js("path\\to\\file");
        assert!(js.contains("t.textContent='path\\\\to\\\\file'"));
    }

    #[test]
    fn single_quote_is_escaped() {
        let js = show_toast_js("it's a test");
        assert!(js.contains("t.textContent='it\\'s a test'"));
    }

    #[test]
    fn newline_is_escaped() {
        let js = show_toast_js("line1\nline2");
        assert!(js.contains("t.textContent='line1\\nline2'"));
    }

    #[test]
    fn carriage_return_is_escaped() {
        let js = show_toast_js("line1\rline2");
        assert!(js.contains("t.textContent='line1\\rline2'"));
    }

    #[test]
    fn multiple_special_chars_combined() {
        let msg = "it's a\\new line\r\nwith tabs";
        let js = show_toast_js(msg);
        assert!(js.contains("\\\\"));
        assert!(js.contains("\\'"));
        assert!(js.contains("\\n"));
        assert!(js.contains("\\r"));
    }

    #[test]
    fn double_quotes_are_not_escaped() {
        let js = show_toast_js("say \"hello\"");
        assert!(js.contains("t.textContent='say \"hello\"'"));
    }

    #[test]
    fn unicode_passes_through() {
        let js = show_toast_js("Hello World");
        assert!(js.contains("t.textContent='Hello World'"));
    }

    #[test]
    fn emoji_passes_through() {
        let js = show_toast_js("Screenshot saved");
        assert!(js.contains("t.textContent='Screenshot saved'"));
    }

    #[test]
    fn output_appends_to_body() {
        let js = show_toast_js("x");
        assert!(js.contains("document.body.appendChild(t)"));
    }

    #[test]
    fn output_has_fade_out_timeout() {
        let js = show_toast_js("x");
        assert!(js.contains("setTimeout"));
        assert!(js.contains("opacity"));
    }

    #[test]
    fn output_removes_element_after_timeout() {
        let js = show_toast_js("x");
        assert!(js.contains("t.remove()"));
    }

    #[test]
    fn output_removes_existing_toast_before_creating() {
        let js = show_toast_js("x");
        assert!(js.contains("document.getElementById('chromeless-toast')"));
        assert!(js.contains("if(e)e.remove()"));
    }

    #[test]
    fn output_has_z_index_max() {
        let js = show_toast_js("x");
        assert!(js.contains("z-index:2147483647"));
    }

    #[test]
    fn output_is_not_pointer_events() {
        let js = show_toast_js("x");
        assert!(js.contains("pointer-events:none"));
    }

    #[test]
    fn long_message_works() {
        let msg = "A".repeat(1000);
        let js = show_toast_js(&msg);
        assert!(js.contains(&format!("t.textContent='{}'", msg)));
    }

    #[test]
    fn output_is_pure_js_no_html() {
        let js = show_toast_js("<script>alert(1)</script>");
        // textContent safely escapes HTML - the <script> appears only inside textContent='...'
        assert!(js.contains("t.textContent='<script>alert(1)</script>'"));
        // Verify it uses textContent (safe) not innerHTML (unsafe)
        assert!(js.contains("textContent="));
        assert!(!js.contains("innerHTML="));
    }

    #[test]
    fn backslash_before_quote() {
        let js = show_toast_js("\\'");
        assert!(js.contains("t.textContent='\\\\\\''"));
    }
}
