<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Package>
  <Name>fin</Name>
  <Version>0.2.27</Version>
  <Summary>Finë: a simple GTK4-based session controller for Linux desktops</Summary>
  <Description>
    Finë is a GTK4-based logout manager designed for simplicity and ease-of-use on Linux desktops.
  </Description>
  <License>MIT</License>
  <Group>Utility</Group>
  <URL>https://github.com/hakimjonas/fin</URL>
  <Icon>/usr/share/icons/fin.png</Icon>
  <RuntimeDependencies>
    <Dependency>gtk4</Dependency>
    <Dependency>glib2</Dependency>
    <Dependency>shared-mime-info</Dependency>
  </RuntimeDependencies>
  <Install>
    <Copy from="target/release/fin" to="/usr/bin/fin" mode="755"/>
    <Copy from="assets/config.toml" to="/usr/share/fin/config.toml" mode="644"/>
    <Copy from="assets/style.css" to="/usr/share/fin/style.css" mode="644"/>
    <Copy from="assets/default.toml" to="/usr/share/fin/themes/default.toml" mode="644"/>
    <Copy from="assets/fin.desktop" to="/usr/share/applications/fin.desktop" mode="644"/>
  </Install>
</Package>