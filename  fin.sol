<?xml version="1.0" encoding="utf-8"?>
<Package>
  <Name>fin</Name>
  <Version>0.1.0</Version>
  <Summary>Finë: a simple GTK4-based session controller for Linux desktops</Summary>
  <Description><![CDATA[
    Finë is a GTK4-based logout manager designed for simplicity and ease-of-use on Linux desktops.
  ]]></Description>
  <License>MIT</License>
  <Group>Utility</Group>
  <URL>https://github.com/yourusername/fin</URL>
  <Icon>fin</Icon>
  <Depends>gtk4</Depends>
  <Install>
    <Copy from="target/release/fin" to="/usr/local/bin/fin" mode="755" />
    <Copy from="assets/config.toml" to="/usr/share/fin/config.toml" mode="644" />
    <Copy from="assets/style.css" to="/usr/share/fin/style.css" mode="644" />
  </Install>
</Package>
