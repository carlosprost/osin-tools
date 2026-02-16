use crate::models::{OsintConfig, OsintResult};
use dns_lookup::lookup_host;
use reqwest::{Client, Proxy};
use std::fs::File;
use std::io::BufReader;
use tauri::async_runtime::spawn_blocking;
use tokio::process::Command as AsyncCommand;

/// Obtiene un cliente HTTP configurado con proxy si está presente en la configuración.
async fn get_http_client(config: &OsintConfig) -> Client {
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true); // Útil para algunos sitios .onion si fuera necesario

    // Usar proxy desde la configuración
    if !config.proxy_url.is_empty() {
        if let Ok(proxy) = Proxy::all(&config.proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    builder.build().unwrap_or_else(|_| Client::new())
}

/// Valida si un target (IP o Dominio) es seguro para ser usado en comandos de sistema.
/// Previene inyecciones de comandos básicos.
fn is_safe_target(target: &str) -> bool {
    // Solo permitir caracteres alfanuméricos, puntos, guiones y dos puntos (para IPv6)
    target
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
        && !target.is_empty()
}

pub async fn perform_ping(target: &str) -> OsintResult {
    if !is_safe_target(target) {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("Target inválido o potencialmente malicioso identificado.".into()),
        };
    }

    let output = AsyncCommand::new("ping")
        .args(["-n", "4", target])
        .output()
        .await;

    match output {
        Ok(out) => {
            let result = String::from_utf8_lossy(&out.stdout).to_string();
            OsintResult {
                success: out.status.success(),
                data: result,
                error: if out.status.success() {
                    None
                } else {
                    Some("La solicitud de eco (ping) no pudo completarse.".into())
                },
            }
        }
        Err(e) => {
            eprintln!("ERROR [system]: Internal ping failure: {}", e);
            OsintResult {
                success: false,
                data: String::new(),
                error: Some("Ocurrió un error interno al ejecutar el comando de red.".into()),
            }
        }
    }
}

pub async fn perform_whois(target: &str, config: &OsintConfig) -> OsintResult {
    let parts: Vec<&str> = target.split('.').collect();
    if parts.len() < 2 {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("Invalid domain".into()),
        };
    }

    let client = get_http_client(config).await;
    let res = client
        .get(format!("https://networkcalc.com/api/dns/lookup/{}", target))
        .send()
        .await;

    match res {
        Ok(resp) => match resp.text().await {
            Ok(text) => OsintResult {
                success: true,
                data: text,
                error: None,
            },
            Err(e) => OsintResult {
                success: false,
                data: "".into(),
                error: Some(e.to_string()),
            },
        },
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("HTTP Whois failed: {}", e)),
        },
    }
}

pub async fn perform_dns_lookup(target: &str) -> OsintResult {
    let target_owned = target.to_string();
    let result = spawn_blocking(move || lookup_host(&target_owned)).await;

    match result {
        Ok(lookup_res) => match lookup_res {
            Ok(ips) => {
                let ip_strings: Vec<String> = ips.iter().map(|ip| ip.to_string()).collect();
                OsintResult {
                    success: true,
                    data: ip_strings.join("\n"),
                    error: None,
                }
            }
            Err(e) => OsintResult {
                success: false,
                data: "".into(),
                error: Some(e.to_string()),
            },
        },
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("Task execution failed: {}", e)),
        },
    }
}

pub async fn extract_metadata(path: String) -> OsintResult {
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR [metadata]: Failed to open file: {} - {}", path, e);
            return OsintResult {
                success: false,
                data: "".into(),
                error: Some("No se pudo abrir el archivo especificado.".into()),
            };
        }
    };

    let mut reader = BufReader::new(file);
    let exifreader = exif::Reader::new();

    match exifreader.read_from_container(&mut reader) {
        Ok(exif) => {
            let mut data = String::new();
            for f in exif.fields() {
                data.push_str(&format!(
                    "{} ({}): {}\n",
                    f.tag,
                    f.ifd_num,
                    f.display_value().with_unit(&exif)
                ));
            }
            OsintResult {
                success: true,
                data,
                error: None,
            }
        }
        Err(e) => {
            eprintln!("ERROR [metadata]: EXIF read failure: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some(
                    "No se encontraron metadatos EXIF o el formato no es soportado.".into(),
                ),
            }
        }
    }
}

pub async fn web_scrape_search(query: String, config: &OsintConfig) -> OsintResult {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(&query)
    );

    let client = get_http_client(config).await;

    match client.get(&url).send().await {
        Ok(res) => {
            if let Ok(html) = res.text().await {
                let document = scraper::Html::parse_document(&html);
                let result_selector = scraper::Selector::parse(".result").unwrap();
                let title_selector = scraper::Selector::parse(".result__title").unwrap();
                let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();
                let link_selector = scraper::Selector::parse(".result__url").unwrap();

                let mut output = format!("🔍 **Resultados de Búsqueda para '{}':**\n\n", query);
                let mut count = 0;

                for element in document.select(&result_selector) {
                    if count >= 5 {
                        break;
                    }

                    let title = element
                        .select(&title_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    let snippet = element
                        .select(&snippet_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    let link = element
                        .select(&link_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();

                    if !title.is_empty() {
                        output.push_str(&format!("**[{}]**\n{}\n🔗 {}\n\n", title, snippet, link));
                        count += 1;
                    }
                }

                if count == 0 {
                    output.push_str("No se encontraron resultados o el scraping fue bloqueado/cambió el formato.");
                }

                OsintResult {
                    success: true,
                    data: output,
                    error: None,
                }
            } else {
                eprintln!("ERROR [scraper]: Text read failure from DuckDuckGO");
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("No se pudo leer la respuesta del motor de búsqueda.".into()),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [scraper]: DuckDuckGO request failed: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some("La conexión con el motor de búsqueda falló.".into()),
            }
        }
    }
}

pub async fn search_username(username: String, config: &OsintConfig) -> OsintResult {
    let client = get_http_client(config).await;
    let mut results = format!("🔍 **Búsqueda de Usuario para '{}':**\n\n", username);

    let platforms = vec![
        ("GitHub", format!("https://github.com/{}", username)),
        ("Twitter", format!("https://twitter.com/{}", username)),
        ("Instagram", format!("https://instagram.com/{}", username)),
        ("Reddit", format!("https://reddit.com/user/{}", username)),
        ("TikTok", format!("https://tiktok.com/@{}", username)),
        (
            "Steam",
            format!("https://steamcommunity.com/id/{}", username),
        ),
    ];

    for (name, url) in platforms {
        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    results.push_str(&format!("✅ {}: Encontrado ( {} )\n", name, url));
                } else if status == 404 {
                    // No encontrado
                } else {
                    results.push_str(&format!(
                        "❓ {}: Código {} (Posible bloqueo)\n",
                        name, status
                    ));
                }
            }
            Err(_) => results.push_str(&format!("❌ {}: Error de conexión.\n", name)),
        }
    }

    OsintResult {
        success: true,
        data: results,
        error: None,
    }
}

pub async fn virus_total_scan(target: String, config: &OsintConfig) -> OsintResult {
    if config.virustotal.is_empty() {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("API Key de VirusTotal no configurada en ajustes.".into()),
        };
    }

    let client = get_http_client(config).await;
    let url = format!("https://www.virustotal.com/api/v3/search?query={}", target);

    match client
        .get(&url)
        .header("x-apikey", &config.virustotal)
        .send()
        .await
    {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                OsintResult {
                    success: true,
                    data: text,
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("No se pudo leer la respuesta de VT".into()),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [virustotal]: Connection failed: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some(
                    "Ocurrió un problema al conectar con el servicio de análisis de reputación."
                        .into(),
                ),
            }
        }
    }
}

pub async fn ip_intel(ip: String, config: &OsintConfig) -> OsintResult {
    let client = get_http_client(config).await;
    // Usamos ip-api.com (limite 45/min para free)
    let url = format!("http://ip-api.com/json/{}?fields=66842623", ip);

    match client.get(&url).send().await {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                OsintResult {
                    success: true,
                    data: text,
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("Error leyendo IP Intel".into()),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [ip_intel]: Request failed: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some("La consulta de geolocalización e inteligencia de IP falló.".into()),
            }
        }
    }
}

pub async fn browse_url(url: String, config: &OsintConfig) -> OsintResult {
    let config = config.clone();
    let proxy_arg = if !config.proxy_url.is_empty() {
        // Chrome usa socks5:// sin la 'h' para el argumento de proxy
        Some(format!(
            "--proxy-server={}",
            config.proxy_url.replace("socks5h://", "socks5://")
        ))
    } else {
        None
    };

    let result = spawn_blocking(move || {
        use headless_chrome::{Browser, LaunchOptions};
        use std::time::Duration;

        let mut args = vec!["--no-sandbox", "--disable-setuid-sandbox"];
        let proxy_str: String;
        if let Some(p) = proxy_arg {
            proxy_str = p;
            args.push(&proxy_str);
        }

        let launch_options = LaunchOptions {
            headless: true,
            window_size: Some((1280, 900)),
            args: args.iter().map(|s| std::ffi::OsStr::new(s)).collect(),
            ..Default::default()
        };

        let browser = match Browser::new(launch_options) {
            Ok(b) => b,
            Err(e) => return Err(format!("No se pudo iniciar Chrome: {}", e)),
        };

        let tab = match browser.new_tab() {
            Ok(t) => t,
            Err(e) => return Err(format!("No se pudo abrir pestaña: {}", e)),
        };

        if let Err(e) = tab.navigate_to(&url) {
            return Err(format!("Error navegando a {}: {}", url, e));
        }

        // --- INYECCIÓN DE COOKIES PARA OSINT AUTENTICADO ---
        use headless_chrome::protocol::cdp::Network::CookieParam;

        let mut cookies_to_set = Vec::new();
        let url_lower = url.to_lowercase();

        if url_lower.contains("linkedin.com") && !config.linkedin_session.is_empty() {
            cookies_to_set.push(CookieParam {
                name: "li_at".into(),
                value: config.linkedin_session.clone().into(),
                domain: Some(".www.linkedin.com".into()),
                path: Some("/".into()),
                secure: Some(true),
                http_only: Some(true),
                same_site: None,
                expires: None,
                priority: None,
                same_party: None,
                source_scheme: None,
                source_port: None,
                partition_key: None,
                url: None,
            });
        } else if (url_lower.contains("instagram.com") || url_lower.contains("instagr.am"))
            && !config.instagram_session.is_empty()
        {
            cookies_to_set.push(CookieParam {
                name: "sessionid".into(),
                value: config.instagram_session.clone().into(),
                domain: Some(".instagram.com".into()),
                path: Some("/".into()),
                secure: Some(true),
                http_only: Some(true),
                same_site: None,
                expires: None,
                priority: None,
                same_party: None,
                source_scheme: None,
                source_port: None,
                partition_key: None,
                url: None,
            });
        } else if (url_lower.contains("twitter.com") || url_lower.contains("x.com"))
            && !config.twitter_session.is_empty()
        {
            cookies_to_set.push(CookieParam {
                name: "auth_token".into(),
                value: config.twitter_session.clone().into(),
                domain: Some(".x.com".into()),
                path: Some("/".into()),
                secure: Some(true),
                http_only: Some(true),
                same_site: None,
                expires: None,
                priority: None,
                same_party: None,
                source_scheme: None,
                source_port: None,
                partition_key: None,
                url: None,
            });
        }

        if !cookies_to_set.is_empty() {
            if let Err(e) = tab.set_cookies(cookies_to_set) {
                eprintln!("Advertencia: Falló inyección de cookies: {}", e);
            } else {
                // Recargar para que las cookies surtan efecto en la sesión
                let _ = tab.navigate_to(&url);
            }
        }
        // --------------------------------------------------

        std::thread::sleep(Duration::from_secs(4));

        let text = match tab.evaluate("document.body.innerText", false) {
            Ok(result) => result
                .value
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "No se pudo extraer texto de la página.".to_string()),
            Err(e) => return Err(format!("Error extrayendo texto: {}", e)),
        };

        let truncated = if text.len() > 4000 {
            format!(
                "{}...\n\n[Texto truncado, {} caracteres totales]",
                &text[..4000],
                text.len()
            )
        } else {
            text
        };

        Ok(truncated)
    })
    .await;

    match result {
        Ok(Ok(text)) => OsintResult {
            success: true,
            data: text,
            error: None,
        },
        Ok(Err(e)) => {
            eprintln!("ERROR [browser]: Scraping failure: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some("No se pudo extraer la información de la página. El sitio podría estar bloqueando el acceso automatizado.".into()),
            }
        }
        Err(e) => {
            eprintln!("ERROR [browser]: Task panic or join error: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some("El motor de navegación experimentó un error crítico interno.".into()),
            }
        }
    }
}

pub async fn generate_dorks(name: String) -> OsintResult {
    let dorks = vec![
        format!("site:linkedin.com \"{}\"", name),
        format!("site:facebook.com \"{}\"", name),
        format!("site:twitter.com \"{}\"", name),
        format!("site:instagram.com \"{}\"", name),
        format!(
            "\"{}\" filetype:pdf OR filetype:doc OR filetype:xls OR filetype:docx",
            name
        ),
        format!(
            "\"{}\" intext:\"dni\" OR intext:\"cuil\" OR intext:\"email\" OR intext:\"correo\"",
            name
        ),
        format!(
            "\"{}\" intext:\"teléfono\" OR intext:\"celular\" OR intext:\"whatsapp\"",
            name
        ),
        format!("\"{}\" \"currículum\" OR \"CV\" OR \"resume\"", name),
        format!(
            "\"{}\" \"dirección\" OR \"domicilio\" OR \"vivienda\"",
            name
        ),
        format!(
            "\"{}\" \"password\" OR \"leak\" OR \"breach\" OR \"filtración\"",
            name
        ),
    ];

    OsintResult {
        success: true,
        data: dorks.join("\n"),
        error: None,
    }
}

pub async fn search_leaks(target: String, config: &OsintConfig) -> OsintResult {
    let client = get_http_client(config).await;
    let api_key = &config.hibp_api_key;

    if api_key.is_empty() {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("API Key de HIBP no configurada. Esta herramienta requiere una llave válida para funcionar. No se realizarán simulaciones para garantizar la integridad del informe.".into()),
        };
    }

    match client
        .get(format!(
            "https://haveibeenpwned.com/api/v3/breachedaccount/{}",
            target
        ))
        .header("hibp-api-key", api_key)
        .send()
        .await
    {
        Ok(res) => {
            let status = res.status();
            if status == 200 {
                let data = res.text().await.unwrap_or_default();
                OsintResult {
                    success: true,
                    data,
                    error: None,
                }
            } else if status == 404 {
                OsintResult {
                    success: true,
                    data: "No se encontraron filtraciones conocidas.".into(),
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some(format!("HIBP API Error: {}", status)),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [hibp]: API Request failure: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some(
                    "No se pudo completar la búsqueda de filtraciones en la base de datos externa."
                        .into(),
                ),
            }
        }
    }
}

pub async fn social_search(target: String, config: &OsintConfig) -> OsintResult {
    let client = get_http_client(config).await;
    let mut results = format!("🔍 **Barrido de Redes para '{}':**\n\n", target);

    let platforms = vec![
        (
            "LinkedIn",
            format!(
                "https://www.google.com/search?q=site:linkedin.com/in/+\"{}\"",
                urlencoding::encode(&target)
            ),
        ),
        (
            "GitHub",
            format!(
                "https://github.com/search?q={}&type=users",
                urlencoding::encode(&target)
            ),
        ),
        (
            "Twitter",
            format!(
                "https://twitter.com/search?q={}",
                urlencoding::encode(&target)
            ),
        ),
    ];

    for (name, url) in platforms {
        match client.get(&url).send().await {
            Ok(_) => results.push_str(&format!(
                "✅ {}: Búsqueda iniciada (Ver resultados en {} )\n",
                name, url
            )),
            Err(_) => results.push_str(&format!("❌ {}: Error al conectar.\n", name)),
        }
    }

    OsintResult {
        success: true,
        data: results,
        error: None,
    }
}

pub async fn dark_search(query: String, config: &OsintConfig) -> OsintResult {
    let client = get_http_client(config).await;
    // Ahmia es un motor de búsqueda que indexa la Dark Web (.onion)
    let url = format!("https://ahmia.fi/search/?q={}", urlencoding::encode(&query));

    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                // Scrape básico de los resultados
                let document = scraper::Html::parse_document(&text);
                let selector = scraper::Selector::parse("li.result").unwrap();
                let mut results = Vec::new();

                for element in document.select(&selector) {
                    if let Some(link) = element
                        .select(&scraper::Selector::parse("cite").unwrap())
                        .next()
                    {
                        results.push(link.text().collect::<String>());
                    }
                }

                if results.is_empty() {
                    return OsintResult {
                        success: true,
                        data: "No se encontraron resultados en la Dark Web para esta consulta."
                            .into(),
                        error: None,
                    };
                }

                OsintResult {
                    success: true,
                    data: format!("Resultados Dark Web (vía Ahmia):\n{}", results.join("\n")),
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("Error leyendo respuesta de Ahmia".into()),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [dark_search]: Ahmia connection failed: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some("La conexión con el nodo de búsqueda Onion falló.".into()),
            }
        }
    }
}

pub async fn shodan_intel(ip: String, config: &OsintConfig) -> OsintResult {
    if config.shodan.is_empty() {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("API Key de Shodan no configurada.".into()),
        };
    }

    let client = get_http_client(config).await;
    let url = format!(
        "https://api.shodan.io/shodan/host/{}?key={}",
        ip, config.shodan
    );

    match client.get(&url).send().await {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                if text.contains("Requires membership") {
                    return OsintResult {
                        success: false,
                        data: "".into(),
                        error: Some("Error de Shodan: Tu API Key es válida pero esta consulta requiere una membresía paga (Membership/Pro).".into()),
                    };
                }
                OsintResult {
                    success: true,
                    data: text,
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("Error leyendo Shodan Intel".into()),
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR [shodan]: API request failed: {}", e);
            OsintResult {
                success: false,
                data: "".into(),
                error: Some(
                    "Hubo un problema al consultar la base de datos de dispositivos conectados."
                        .into(),
                ),
            }
        }
    }
}
