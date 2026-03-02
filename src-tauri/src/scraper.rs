use headless_chrome::{Browser, LaunchOptions};
use std::time::Duration;

/// Inicializa una instancia de Chromium en modo Headless con parámetros evasivos para evitar detección.
fn init_browser() -> Result<Browser, String> {
    let options = LaunchOptions::default_builder()
        .headless(true)
        .window_size(Some((1366, 768)))
        // User agent realista de Chromium
        .args(vec![
            std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
            std::ffi::OsStr::new("--no-sandbox"),
            std::ffi::OsStr::new("--disable-infobars"),
        ])
        .build()
        .map_err(|e| format!("Failed to create browser options: {:?}", e))?;

    Browser::new(options).map_err(|e| format!("Failed to launch browser: {:?}", e))
}

/// Inyecta scripts antibots y navega a la URL para raspar texto plano limpiando la basura del DOM.
pub fn scrape_url_stealth(url: &str) -> Result<String, String> {
    let browser = init_browser()?;
    let tab = browser
        .new_tab()
        .map_err(|e| format!("Failed to create tab: {:?}", e))?;

    // Script stealth para emular entorno humano y ocultar la bandera `webdriver`.
    let stealth_js = r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        window.chrome = { runtime: {} };
        Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4] });
        Object.defineProperty(navigator, 'languages', { get: () => ['es-AR', 'es', 'en-US', 'en'] });
        
        // Falsificar WebGL para no parecer un entorno de servidor estéril
        const overrideWebGL = () => {
            const getParameter = WebGLRenderingContext.prototype.getParameter;
            WebGLRenderingContext.prototype.getParameter = function(parameter) {
                if (parameter === 37445) return 'Google Inc. (Apple)';
                if (parameter === 37446) return 'ANGLE (Apple, Apple M1 Pro, OpenGL 4.1)';
                return getParameter.apply(this, [parameter]);
            };
        };
        overrideWebGL();
    "#;

    println!("[Scraper Stealth] Navegando a: {}", url);
    tab.navigate_to(url)
        .map_err(|e| format!("Error navegando a '{}': {:?}", url, e))?;

    // Las tácticas modernas a veces requieren inyectar el script apenas el DOM está disponible pero antes del render final.
    let _ = tab.evaluate(stealth_js, false);

    // Esperamos a que la navegación termine
    tab.wait_until_navigated()
        .map_err(|e| format!("Error esperando navegación para '{}': {:?}", url, e))?;

    // Esperar unos segundos extra vitales para cargar SPAs (React, Vue) y evitar Captchas dinámicos iniciales
    std::thread::sleep(Duration::from_secs(4));

    // Limpieza adicional via Readability pura heurística (emulada extrayendo texto)
    let extract_js = r#"
        (function() {
            // Remueve elementos no deseados comunes
            const garbage = document.querySelectorAll('script, style, noscript, iframe, svg, nav, footer, header, .nav, .menu, .sidebar, .ads');
            garbage.forEach(e => e.remove());
            return document.body.innerText;
        })();
    "#;

    let content = tab
        .evaluate(extract_js, false)
        .map_err(|e| format!("Error evaluando contenido extraído en '{}': {:?}", url, e))?;

    let result = content
        .value
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if result.trim().is_empty() {
        return Err(format!(
            "Extracción vacía. Es posible que el sitio haya bloqueado el intento a {}",
            url
        ));
    }

    Ok(result)
}
