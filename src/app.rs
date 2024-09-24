use crate::datatypes::{Closure, StorableValue};
use crate::preprocess::Static;
use crate::state::{init_state, is_fixed_point, tick, State};
use ratatui::layout::Direction;
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
        Self {
            cur_state: init_state(static_info),
            states: vec![],
            static_info,
            source,
            exit: false,
            expand_closures: false,
        }
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
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn next_state(&mut self) {
        if is_fixed_point(&self.cur_state, self.static_info) {
            self.exit = true;
        } else {
            self.states.push(self.cur_state.clone());
            self.cur_state = tick(self.cur_state.clone(), self.static_info).unwrap();
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
            Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)])
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

        for (&local_env_area, local_env) in Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1); self.cur_state.env.len()])
            .vertical_margin(0)
            .horizontal_margin(0)
            .split(env_area)
            .iter()
            .rev()
            .zip(self.cur_state.env.iter())
        {
            Paragraph::new(Text::from(
                local_env
                    .mapping
                    .iter()
                    .map(|(i, val)| {
                        Line::from(format!("{}: ", i).bold().blue() + format!("{:?}", val).into())
                    })
                    .collect::<Vec<_>>(),
            ))
            .block(
                Block::bordered()
                    .title(
                        Title::from(format!("{} Env", local_env.func_name))
                            .alignment(Alignment::Center),
                    )
                    .border_set(border::ROUNDED),
            )
            .render(local_env_area, buf);
        }

        Paragraph::new(Text::from(
            self.cur_state
                .store
                .iter()
                .enumerate()
                .flat_map(|(i, val)| {
                    if let StorableValue::Closure(Closure::Function(lineno, env)) = val {
                        if self.expand_closures {
                            let mut v = vec![Line::from(
                                format!("{}: ", i).bold().blue()
                                    + format!(
                                        "Closure with {} at line {}",
                                        env.iter()
                                            .map(|x| x.func_name.clone())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        lineno
                                    )
                                    .into(),
                            )];

                            v.extend(env.iter().map(|local_env| {
                                Line::from(format!(
                                    "{}: {:?}",
                                    local_env.func_name, local_env.mapping
                                ))
                            }));

                            v
                        } else {
                            vec![Line::from(
                                format!("{}: ", i).bold().blue()
                                    + format!(
                                        "Closure with {} at line {}",
                                        env.iter()
                                            .map(|x| x.func_name.clone())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        lineno
                                    )
                                    .into(),
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
