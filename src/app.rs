use crate::datatypes::{State, StorableValue};
use crate::preprocess::Static;
use crate::state::{init_state, is_fixed_point, tick};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{block::Title, Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use std::io;

pub struct App<'a> {
    cur_state: State,
    states: Vec<State>,
    static_info: &'a Static<'a>,
    source: &'a str,
    exit: bool,
    expand_closures: bool,
}

impl<'a> App<'a> {
    pub fn new(source: &'a str, static_info: &'a Static) -> App<'a> {
        let mut app = Self {
            cur_state: init_state(static_info),
            states: vec![],
            static_info,
            source,
            exit: false,
            expand_closures: false,
        };

        app.states.push(app.cur_state.clone());

        app
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('e') => self.expand_closures = !self.expand_closures,
            KeyCode::Right => self.next_state(),
            KeyCode::Left => self.prev_state(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn next_state(&mut self) {
        match self.states.iter().position(|x| x == &self.cur_state) {
            Some(idx) => {
                if let Some(next_state) = self.states.get(idx + 1) {
                    self.cur_state = next_state.clone();
                } else {
                    if !is_fixed_point(&self.cur_state, self.static_info) {
                        self.cur_state = tick(self.cur_state.clone(), self.static_info).unwrap();
                        self.states.push(self.cur_state.clone());
                    }
                }
            }
            None => panic!("Current state not in the possible states"),
        }
    }

    fn prev_state(&mut self) {
        match self.states.iter().position(|x| x == &self.cur_state) {
            Some(idx) => self.cur_state = self.states[idx.saturating_sub(1)].clone(),
            None => panic!("Current state not in the possible states"),
        }
    }
}

impl<'a> Widget for &App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [title_area, main_area, instructions_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        let [code_area, state_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(main_area);

        // title
        Block::new()
            .title(Title::from(Line::from(vec![" SimpliPy ".green().bold()])))
            .title_alignment(Alignment::Center)
            .render(title_area, buf);

        // source code
        Paragraph::new(Text::from(
            self.source
                .lines()
                .enumerate()
                .map(|(i, x)| {
                    let mut line = Line::from((format!("{:02}: ", i + 1)).blue() + x.into());
                    if i + 1 == self.cur_state.lineno {
                        line = line.on_black();
                    }
                    line
                })
                .collect::<Vec<_>>(),
        ))
        .block(
            Block::bordered()
                .title(Title::from(" Source Code ".bold()).alignment(Alignment::Center))
                .border_set(border::ROUNDED),
        )
        .render(code_area, buf);

        let [var_area, stack_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(state_area);

        let [env_area, store_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(var_area);

        if let Some(local_env) = &self.cur_state.local_env {
            let [local_env_area, global_env_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(env_area);

            Paragraph::new(Text::from(
                local_env
                    .iter()
                    .map(|(i, val)| {
                        Line::from(format!("{}: ", i).bold().blue() + format!("{:?}", val).into())
                    })
                    .collect::<Vec<_>>(),
            ))
            .block(
                Block::bordered()
                    .title(Title::from(format!("Local Env")).alignment(Alignment::Center))
                    .border_set(border::ROUNDED),
            )
            .render(local_env_area, buf);

            Paragraph::new(Text::from(
                self.cur_state
                    .global_env
                    .iter()
                    .map(|(i, val)| {
                        Line::from(format!("{}: ", i).bold().blue() + format!("{:?}", val).into())
                    })
                    .collect::<Vec<_>>(),
            ))
            .block(
                Block::bordered()
                    .title(Title::from(format!("Global Env")).alignment(Alignment::Center))
                    .border_set(border::ROUNDED),
            )
            .render(global_env_area, buf);
        } else {
            Paragraph::new(Text::from(
                self.cur_state
                    .global_env
                    .iter()
                    .map(|(i, val)| {
                        Line::from(format!("{}: ", i).bold().blue() + format!("{:?}", val).into())
                    })
                    .collect::<Vec<_>>(),
            ))
            .block(
                Block::bordered()
                    .title(Title::from(format!("Global Env")).alignment(Alignment::Center))
                    .border_set(border::ROUNDED),
            )
            .render(env_area, buf);
        }

        Paragraph::new(Text::from(
            self.cur_state
                .store
                .iter()
                .enumerate()
                .flat_map(|(i, val)| {
                    if let StorableValue::DefinitionClosure(lineno, env, _) = val {
                        if self.expand_closures {
                            let mut v = vec![Line::from(
                                format!("{}: ", i).bold().blue()
                                    + format!("Closure at line {}", lineno).into(),
                            )];

                            v.extend(
                                env.iter()
                                    .map(|local_env| Line::from(format!("{:?}", local_env))),
                            );

                            v
                        } else {
                            vec![Line::from(
                                format!("{}: ", i).bold().blue()
                                    + format!("Closure at line {}", lineno).into(),
                            )]
                        }
                    } else {
                        vec![Line::from(
                            format!("{}: ", i).bold().blue() + format!("{:?}", val).into(),
                        )]
                    }
                })
                .collect::<Vec<_>>(),
        ))
        .block(
            Block::bordered()
                .title(Title::from("Store").alignment(Alignment::Center))
                .border_set(border::ROUNDED),
        )
        .render(store_area, buf);

        Paragraph::new(Text::from(
            self.cur_state
                .stack
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    Line::from(format!("{}: ", i).bold().blue() + format!("{:?}", val).into())
                })
                .collect::<Vec<_>>(),
        ))
        .block(
            Block::bordered()
                .title(Title::from("Stack").alignment(Alignment::Center))
                .border_set(border::ROUNDED),
        )
        .render(stack_area, buf);

        // instructions
        Block::new()
            .title(Title::from(Line::from(vec![
                " Previous State ".into(),
                "<Left>".blue().bold(),
                " Next State ".into(),
                "<Right>".blue().bold(),
                " Expand Closures ".into(),
                "<E>".blue().bold(),
                " Quit ".into(),
                "<Q> ".blue().bold(),
            ])))
            .title_alignment(Alignment::Center)
            .render(instructions_area, buf);
    }
}
