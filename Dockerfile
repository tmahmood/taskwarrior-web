FROM archlinux:latest

# Install
RUN pacman -Suy --needed --noconfirm sudo curl base-devel git npm python
RUN pacman -S --noconfirm task timew
RUN useradd -m -G wheel builder && passwd -d builder
RUN echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers
RUN chown -R builder:builder /home/builder && chmod -R 775 /home/builder
RUN npm install --global rollup
RUN mkdir -p /usr/share/doc/task/rc/
USER builder
ENV HOME=/home/builder
WORKDIR /home/builder
# Rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
    && chmod +x rustup.sh \
    && ./rustup.sh -y --default-toolchain nightly \
    && source $HOME/.cargo/env

RUN mkdir $HOME/app
# Tailwind
RUN cd $HOME/app && curl -o $HOME/app/tailwindcss -sL https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64 \
    && chmod +x $HOME/app/tailwindcss

# Copy files
COPY ./frontend $HOME/app/frontend
COPY ./src $HOME/app/src
COPY ./build.rs $HOME/app/
COPY ./Cargo.toml $HOME/app/Cargo.toml
COPY ./docker/start.sh $HOME/start.sh
RUN sudo chown -R $(whoami) $HOME/app
RUN cd $HOME/app/frontend && npm install && cd ..
RUN cd $HOME/app && source $HOME/.cargo/env &&cargo build --release

EXPOSE 3000

# Taskwarrior data volume
VOLUME [ "/usr/share/doc/task/rc" ]

ENV TASKRC="$HOME/.taskrc"
ENV TASKDATA="$HOME/.task"

RUN sudo chmod +x $HOME/start.sh

CMD ["/home/builder/start.sh"]