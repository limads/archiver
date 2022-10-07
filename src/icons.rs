/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/

use gtk4::gdk;
use gtk4::*;
use gtk4::prelude::*;
use std::collections::HashMap;
use gdk_pixbuf::Pixbuf;

pub fn load_icons_as_pixbufs_from_resource(res_root : &str, icons : &[&'static str]) -> Result<HashMap<&'static str, Pixbuf>, String> {
    if let Some(display) = gdk::Display::default() {
        let theme = IconTheme::for_display(&display);
        theme.add_resource_path(res_root);
        theme.add_resource_path(&format!("{}/icons", res_root));
        let mut icon_pixbufs = HashMap::new();
        for icon_name in icons {
            let pxb = Pixbuf::from_resource(&format!("{}/icons/scalable/actions/{}.svg", res_root, icon_name)).unwrap();
            icon_pixbufs.insert(*icon_name,pxb);
        }
        Ok(icon_pixbufs)
        // } else {
        //    Err(format!("No icon theme for default GDK display"))
        // }
    } else {
        Err(format!("No default GDK display"))
    }
}

pub fn load_icons_as_pixbufs_from_paths(icons : &[&'static str]) -> Result<HashMap<&'static str, Pixbuf>, String> {
    if let Some(display) = gdk::Display::default() {
        let theme = IconTheme::for_display(&display);
        let mut icon_pixbufs = HashMap::new();
        for icon_name in icons {
            let icon = theme.lookup_icon(icon_name, &[], 16, 1, TextDirection::Ltr, IconLookupFlags::empty());
            let path = icon.file()
                .ok_or(format!("Icon {} has no corresponing file", icon_name))?
                .path()
                .ok_or(format!("File for icon {} has no valid path", icon_name))?;
                let pxb = Pixbuf::from_file_at_scale(path, 16, 16, true).unwrap();
                icon_pixbufs.insert(*icon_name,pxb);
            //} else {
            //    return Err(format!("No icon named {}", icon_name));
            //}
        }
        Ok(icon_pixbufs)
        // } else {
        //    Err(format!("No icon theme for default GDK display"))
        // }
    } else {
        Err(format!("No default GDK display"))
    }
}

pub fn read_resource() -> gio::Resource {
    gio::Resource::load("data/resources.gresource").unwrap()
}

