// TBX Translator - themes.rs
// Sistema de temas de cores

use egui::Color32;

#[derive(Debug, Clone, PartialEq)]
pub struct AppTheme {
    pub id: &'static str,
    pub name: &'static str,
    // Principais
    pub base: Color32,       // fundo da janela
    pub mantle: Color32,     // fundo do painel secundário
    pub crust: Color32,      // barra de título
    pub surface0: Color32,   // widgets inativos
    pub surface1: Color32,   // widgets hover
    pub surface2: Color32,   // widgets ativos
    pub overlay0: Color32,   // textos suaves
    pub text: Color32,       // texto principal
    pub accent: Color32,     // cor de destaque / active
    pub accent2: Color32,    // destaque secundário
    pub border: Color32,     // bordas
}

impl AppTheme {
    pub fn get(id: &str) -> AppTheme {
        let all = Self::all();
        all.into_iter().find(|t| t.id == id).unwrap_or_else(|| Self::all().remove(0))
    }

    pub fn all() -> Vec<AppTheme> {
        vec![
            // 1. Catppuccin Mocha (padrão)
            AppTheme {
                id: "catppuccin_mocha",
                name: "Catppuccin Mocha",
                base: Color32::from_rgb(30, 30, 46),
                mantle: Color32::from_rgb(24, 24, 37),
                crust: Color32::from_rgb(17, 17, 27),
                surface0: Color32::from_rgb(49, 50, 68),
                surface1: Color32::from_rgb(69, 71, 90),
                surface2: Color32::from_rgb(88, 91, 112),
                overlay0: Color32::from_rgb(108, 112, 134),
                text: Color32::from_rgb(205, 214, 244),
                accent: Color32::from_rgb(137, 180, 250),
                accent2: Color32::from_rgb(203, 166, 247),
                border: Color32::from_rgb(69, 71, 90),
            },
            // 2. Catppuccin Latte (claro)
            AppTheme {
                id: "catppuccin_latte",
                name: "Catppuccin Latte",
                base: Color32::from_rgb(239, 241, 245),
                mantle: Color32::from_rgb(230, 233, 239),
                crust: Color32::from_rgb(220, 224, 232),
                surface0: Color32::from_rgb(204, 208, 218),
                surface1: Color32::from_rgb(188, 192, 204),
                surface2: Color32::from_rgb(172, 176, 190),
                overlay0: Color32::from_rgb(140, 143, 161),
                text: Color32::from_rgb(76, 79, 105),
                accent: Color32::from_rgb(30, 102, 245),
                accent2: Color32::from_rgb(136, 57, 239),
                border: Color32::from_rgb(172, 176, 190),
            },
            // 3. Nord
            AppTheme {
                id: "nord",
                name: "Nord",
                base: Color32::from_rgb(46, 52, 64),
                mantle: Color32::from_rgb(39, 44, 54),
                crust: Color32::from_rgb(36, 40, 49),
                surface0: Color32::from_rgb(59, 66, 82),
                surface1: Color32::from_rgb(67, 76, 94),
                surface2: Color32::from_rgb(76, 86, 106),
                overlay0: Color32::from_rgb(129, 161, 193),
                text: Color32::from_rgb(229, 233, 240),
                accent: Color32::from_rgb(136, 192, 208),
                accent2: Color32::from_rgb(163, 190, 140),
                border: Color32::from_rgb(67, 76, 94),
            },
            // 4. Dracula
            AppTheme {
                id: "dracula",
                name: "Dracula",
                base: Color32::from_rgb(40, 42, 54),
                mantle: Color32::from_rgb(33, 34, 44),
                crust: Color32::from_rgb(24, 25, 34),
                surface0: Color32::from_rgb(68, 71, 90),
                surface1: Color32::from_rgb(98, 114, 164),
                surface2: Color32::from_rgb(122, 162, 247),
                overlay0: Color32::from_rgb(98, 114, 164),
                text: Color32::from_rgb(248, 248, 242),
                accent: Color32::from_rgb(189, 147, 249),
                accent2: Color32::from_rgb(80, 250, 123),
                border: Color32::from_rgb(68, 71, 90),
            },
            // 5. Gruvbox Dark
            AppTheme {
                id: "gruvbox_dark",
                name: "Gruvbox Dark",
                base: Color32::from_rgb(40, 40, 40),
                mantle: Color32::from_rgb(29, 32, 33),
                crust: Color32::from_rgb(28, 28, 28),
                surface0: Color32::from_rgb(60, 56, 54),
                surface1: Color32::from_rgb(80, 73, 69),
                surface2: Color32::from_rgb(102, 92, 84),
                overlay0: Color32::from_rgb(168, 153, 132),
                text: Color32::from_rgb(235, 219, 178),
                accent: Color32::from_rgb(250, 189, 47),
                accent2: Color32::from_rgb(142, 192, 124),
                border: Color32::from_rgb(80, 73, 69),
            },
            // 6. Tokyo Night
            AppTheme {
                id: "tokyo_night",
                name: "Tokyo Night",
                base: Color32::from_rgb(26, 27, 38),
                mantle: Color32::from_rgb(22, 22, 30),
                crust: Color32::from_rgb(16, 16, 22),
                surface0: Color32::from_rgb(36, 40, 59),
                surface1: Color32::from_rgb(41, 46, 66),
                surface2: Color32::from_rgb(59, 66, 97),
                overlay0: Color32::from_rgb(86, 95, 137),
                text: Color32::from_rgb(169, 177, 214),
                accent: Color32::from_rgb(122, 162, 247),
                accent2: Color32::from_rgb(187, 154, 247),
                border: Color32::from_rgb(41, 46, 66),
            },
            // 7. Solarized Dark
            AppTheme {
                id: "solarized_dark",
                name: "Solarized Dark",
                base: Color32::from_rgb(0, 43, 54),
                mantle: Color32::from_rgb(7, 54, 66),
                crust: Color32::from_rgb(0, 33, 42),
                surface0: Color32::from_rgb(7, 54, 66),
                surface1: Color32::from_rgb(88, 110, 117),
                surface2: Color32::from_rgb(101, 123, 131),
                overlay0: Color32::from_rgb(131, 148, 150),
                text: Color32::from_rgb(147, 161, 161),
                accent: Color32::from_rgb(38, 139, 210),
                accent2: Color32::from_rgb(42, 161, 152),
                border: Color32::from_rgb(7, 54, 66),
            },
            // 8. One Dark
            AppTheme {
                id: "one_dark",
                name: "One Dark",
                base: Color32::from_rgb(40, 44, 52),
                mantle: Color32::from_rgb(33, 37, 43),
                crust: Color32::from_rgb(24, 26, 31),
                surface0: Color32::from_rgb(49, 53, 61),
                surface1: Color32::from_rgb(57, 62, 70),
                surface2: Color32::from_rgb(66, 71, 81),
                overlay0: Color32::from_rgb(92, 99, 112),
                text: Color32::from_rgb(171, 178, 191),
                accent: Color32::from_rgb(97, 175, 239),
                accent2: Color32::from_rgb(198, 120, 221),
                border: Color32::from_rgb(57, 62, 70),
            },
            // 9. Monokai
            AppTheme {
                id: "monokai",
                name: "Monokai",
                base: Color32::from_rgb(39, 40, 34),
                mantle: Color32::from_rgb(30, 31, 25),
                crust: Color32::from_rgb(20, 21, 16),
                surface0: Color32::from_rgb(62, 61, 50),
                surface1: Color32::from_rgb(75, 73, 60),
                surface2: Color32::from_rgb(117, 113, 94),
                overlay0: Color32::from_rgb(117, 113, 94),
                text: Color32::from_rgb(248, 248, 242),
                accent: Color32::from_rgb(249, 38, 114),
                accent2: Color32::from_rgb(102, 217, 239),
                border: Color32::from_rgb(75, 73, 60),
            },
            // 10. Ayu Dark
            AppTheme {
                id: "ayu_dark",
                name: "Ayu Dark",
                base: Color32::from_rgb(13, 16, 23),
                mantle: Color32::from_rgb(10, 13, 19),
                crust: Color32::from_rgb(7, 9, 14),
                surface0: Color32::from_rgb(21, 28, 39),
                surface1: Color32::from_rgb(26, 34, 47),
                surface2: Color32::from_rgb(33, 43, 59),
                overlay0: Color32::from_rgb(74, 98, 139),
                text: Color32::from_rgb(201, 202, 224),
                accent: Color32::from_rgb(255, 179, 84),
                accent2: Color32::from_rgb(57, 186, 230),
                border: Color32::from_rgb(26, 34, 47),
            },
            // 11. Rosé Pine
            AppTheme {
                id: "rose_pine",
                name: "Rosé Pine",
                base: Color32::from_rgb(25, 23, 36),
                mantle: Color32::from_rgb(30, 28, 43),
                crust: Color32::from_rgb(21, 19, 31),
                surface0: Color32::from_rgb(64, 61, 82),
                surface1: Color32::from_rgb(110, 106, 134),
                surface2: Color32::from_rgb(144, 140, 170),
                overlay0: Color32::from_rgb(110, 106, 134),
                text: Color32::from_rgb(224, 222, 244),
                accent: Color32::from_rgb(235, 188, 186),
                accent2: Color32::from_rgb(156, 207, 216),
                border: Color32::from_rgb(64, 61, 82),
            },
            // 12. Kanagawa
            AppTheme {
                id: "kanagawa",
                name: "Kanagawa",
                base: Color32::from_rgb(22, 22, 29),
                mantle: Color32::from_rgb(26, 27, 38),
                crust: Color32::from_rgb(19, 19, 26),
                surface0: Color32::from_rgb(54, 54, 73),
                surface1: Color32::from_rgb(84, 84, 109),
                surface2: Color32::from_rgb(110, 110, 155),
                overlay0: Color32::from_rgb(84, 84, 109),
                text: Color32::from_rgb(220, 215, 186),
                accent: Color32::from_rgb(126, 156, 216),
                accent2: Color32::from_rgb(152, 187, 108),
                border: Color32::from_rgb(54, 54, 73),
            },
            // 13. Everforest
            AppTheme {
                id: "everforest",
                name: "Everforest",
                base: Color32::from_rgb(45, 53, 45),
                mantle: Color32::from_rgb(38, 44, 38),
                crust: Color32::from_rgb(30, 35, 30),
                surface0: Color32::from_rgb(65, 74, 65),
                surface1: Color32::from_rgb(90, 103, 90),
                surface2: Color32::from_rgb(109, 124, 109),
                overlay0: Color32::from_rgb(157, 169, 157),
                text: Color32::from_rgb(211, 198, 170),
                accent: Color32::from_rgb(131, 192, 146),
                accent2: Color32::from_rgb(249, 166, 109),
                border: Color32::from_rgb(90, 103, 90),
            },
            // 14. Palenight
            AppTheme {
                id: "palenight",
                name: "Palenight",
                base: Color32::from_rgb(41, 45, 62),
                mantle: Color32::from_rgb(33, 37, 53),
                crust: Color32::from_rgb(26, 29, 43),
                surface0: Color32::from_rgb(58, 63, 84),
                surface1: Color32::from_rgb(75, 82, 107),
                surface2: Color32::from_rgb(95, 103, 134),
                overlay0: Color32::from_rgb(103, 110, 149),
                text: Color32::from_rgb(166, 172, 205),
                accent: Color32::from_rgb(130, 170, 255),
                accent2: Color32::from_rgb(199, 146, 234),
                border: Color32::from_rgb(75, 82, 107),
            },
            // 15. Midnight Purple
            AppTheme {
                id: "midnight_purple",
                name: "Midnight Purple",
                base: Color32::from_rgb(18, 12, 28),
                mantle: Color32::from_rgb(14, 8, 22),
                crust: Color32::from_rgb(10, 5, 16),
                surface0: Color32::from_rgb(40, 28, 60),
                surface1: Color32::from_rgb(60, 44, 88),
                surface2: Color32::from_rgb(80, 60, 116),
                overlay0: Color32::from_rgb(120, 90, 160),
                text: Color32::from_rgb(220, 210, 240),
                accent: Color32::from_rgb(180, 130, 255),
                accent2: Color32::from_rgb(255, 130, 200),
                border: Color32::from_rgb(60, 44, 88),
            },
            // 16. Cyber Neon
            AppTheme {
                id: "cyber_neon",
                name: "Cyber Neon",
                base: Color32::from_rgb(10, 10, 18),
                mantle: Color32::from_rgb(7, 7, 14),
                crust: Color32::from_rgb(4, 4, 10),
                surface0: Color32::from_rgb(20, 20, 35),
                surface1: Color32::from_rgb(30, 30, 50),
                surface2: Color32::from_rgb(45, 45, 70),
                overlay0: Color32::from_rgb(80, 80, 120),
                text: Color32::from_rgb(220, 240, 255),
                accent: Color32::from_rgb(0, 255, 200),
                accent2: Color32::from_rgb(255, 50, 130),
                border: Color32::from_rgb(0, 180, 140),
            },
            // 17. Ocean Breeze
            AppTheme {
                id: "ocean_breeze",
                name: "Ocean Breeze",
                base: Color32::from_rgb(15, 35, 50),
                mantle: Color32::from_rgb(10, 26, 40),
                crust: Color32::from_rgb(6, 18, 30),
                surface0: Color32::from_rgb(25, 58, 84),
                surface1: Color32::from_rgb(38, 82, 115),
                surface2: Color32::from_rgb(52, 108, 150),
                overlay0: Color32::from_rgb(80, 145, 190),
                text: Color32::from_rgb(200, 230, 248),
                accent: Color32::from_rgb(80, 200, 220),
                accent2: Color32::from_rgb(120, 240, 180),
                border: Color32::from_rgb(38, 82, 115),
            },
            // 18. Sunset
            AppTheme {
                id: "sunset",
                name: "Sunset",
                base: Color32::from_rgb(30, 18, 30),
                mantle: Color32::from_rgb(22, 12, 22),
                crust: Color32::from_rgb(16, 8, 16),
                surface0: Color32::from_rgb(55, 32, 50),
                surface1: Color32::from_rgb(80, 48, 72),
                surface2: Color32::from_rgb(105, 64, 95),
                overlay0: Color32::from_rgb(150, 100, 130),
                text: Color32::from_rgb(245, 220, 210),
                accent: Color32::from_rgb(255, 120, 70),
                accent2: Color32::from_rgb(255, 200, 80),
                border: Color32::from_rgb(80, 48, 72),
            },
            // 19. Mango
            AppTheme {
                id: "mango",
                name: "Mango",
                base: Color32::from_rgb(28, 22, 10),
                mantle: Color32::from_rgb(20, 16, 6),
                crust: Color32::from_rgb(14, 10, 3),
                surface0: Color32::from_rgb(58, 46, 18),
                surface1: Color32::from_rgb(84, 68, 28),
                surface2: Color32::from_rgb(110, 90, 40),
                overlay0: Color32::from_rgb(160, 130, 70),
                text: Color32::from_rgb(248, 236, 200),
                accent: Color32::from_rgb(255, 168, 30),
                accent2: Color32::from_rgb(120, 210, 80),
                border: Color32::from_rgb(84, 68, 28),
            },
            // 20. Terminal Green (hacker)
            AppTheme {
                id: "terminal_green",
                name: "Terminal Green",
                base: Color32::from_rgb(8, 16, 8),
                mantle: Color32::from_rgb(4, 10, 4),
                crust: Color32::from_rgb(2, 6, 2),
                surface0: Color32::from_rgb(16, 36, 16),
                surface1: Color32::from_rgb(24, 56, 24),
                surface2: Color32::from_rgb(32, 76, 32),
                overlay0: Color32::from_rgb(60, 120, 60),
                text: Color32::from_rgb(160, 240, 160),
                accent: Color32::from_rgb(0, 255, 60),
                accent2: Color32::from_rgb(0, 200, 180),
                border: Color32::from_rgb(24, 56, 24),
            },
        ]
    }
}
