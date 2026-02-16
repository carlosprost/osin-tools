// src-tauri/src/mac_spoof.rs
use rand::Rng;
use std::process::Command;

pub fn generate_random_mac() -> String {
    let mut rng = rand::thread_rng();
    let mut mac = vec![0u8; 6];
    rng.fill(&mut mac[..]);
    mac[0] = (mac[0] & 0xFC) | 0x02; // Set local bit, unset multicast bit

    mac.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join("")
}

pub fn get_active_adapter_info() -> Result<(String, String), String> {
    let output = Command::new("powershell")
        .args([
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | Select-Object -First 1 -ExpandProperty Name"
        ])
        .output()
        .map_err(|e| format!("Error ejecutando PS (get_name): {}", e))?;

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        return Err(
            "No se encontró un adaptador activo (asegurate de estar conectado a internet)".into(),
        );
    }

    let output_id = Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!("(Get-NetAdapter -Name '{}').InterfaceDescription", name),
        ])
        .output()
        .map_err(|e| format!("Error ejecutando PS (get_desc): {}", e))?;

    let desc = String::from_utf8_lossy(&output_id.stdout)
        .trim()
        .to_string();

    Ok((name, desc))
}

pub fn apply_mac_spoof(adapter_name: &str, new_mac: &str) -> Result<(), String> {
    let script = format!(
        r#"
        $adapterName = "{}"
        $newMac = "{}"
        $path = "HKLM:\SYSTEM\CurrentControlSet\Control\Class\{{4d36e972-e325-11ce-bfc1-08002be10318}}"
        $found = $false
        Get-ChildItem $path | ForEach-Object {{
            $val = Get-ItemProperty $_.PSPath
            if ($val.DriverDesc -eq (Get-NetAdapter -Name $adapterName).InterfaceDescription) {{
                Set-ItemProperty -Path $_.PSPath -Name "NetworkAddress" -Value $newMac
                $found = $true
            }}
        }}
        if ($found) {{
            Disable-NetAdapter -Name $adapterName -Confirm:$false
            Enable-NetAdapter -Name $adapterName -Confirm:$false
        }} else {{
            throw "No se encontró la clave de registro para el adaptador: $adapterName"
        }}
        "#,
        adapter_name, new_mac
    );

    let output = Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|e| format!("Error de I/O de sistema: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        let out_msg = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(format!(
            "Fallo PS (Apply):\nErr: {}\nOut: {}",
            err_msg, out_msg
        ));
    }

    Ok(())
}

pub fn reset_mac(adapter_name: &str) -> Result<(), String> {
    let script = format!(
        r#"
        $adapterName = "{}"
        $path = "HKLM:\SYSTEM\CurrentControlSet\Control\Class\{{4d36e972-e325-11ce-bfc1-08002be10318}}"
        Get-ChildItem $path | ForEach-Object {{
            $val = Get-ItemProperty $_.PSPath
            if ($val.DriverDesc -eq (Get-NetAdapter -Name $adapterName).InterfaceDescription) {{
                Remove-ItemProperty -Path $_.PSPath -Name "NetworkAddress" -ErrorAction SilentlyContinue
            }}
        }}
        Disable-NetAdapter -Name $adapterName -Confirm:$false
        Enable-NetAdapter -Name $adapterName -Confirm:$false
        "#,
        adapter_name
    );

    let output = Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|e| format!("Error de I/O de sistema: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("Fallo PS (Reset):\n{}", err_msg));
    }

    Ok(())
}
