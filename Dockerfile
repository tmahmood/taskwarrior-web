FROM archlinux:latest AS rustbase
RUN pacman -Suy --needed --noconfirm curl base-devel npm 
RUN mkdir /app \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
    && chmod +x rustup.sh \
    && ./rustup.sh -y --default-toolchain nightly

FROM rustbase AS buildapp
# Copy files
COPY ./Cargo.toml /app/Cargo.toml
COPY ./frontend /app/frontend
COPY ./src /app/src
COPY ./build.rs /app/
RUN cd /app \
    && source $HOME/.cargo/env \
    && cargo build --release

FROM archlinux:latest
ARG TASK_ADDON_BUGWARRIOR="false"
ARG TASK_ADDON_BUGWARRIOR_FEATURES=""

# Install
RUN echo "NoExtract = !usr/share/doc/timew/*" >> /etc/pacman.conf \
 && echo "NoExtract = !usr/share/doc/task/*" >> /etc/pacman.conf \
 && pacman -Suy --needed --noconfirm git python \
 && pacman -S --noconfirm task timew cronie vi \
 && pacman -S --noconfirm python-pip \
 && useradd -m -d /app task && passwd -d task \
 && mkdir -p /app/bin \
 && mkdir -p /app/taskdata \
 && mkdir -p /app/.task/hooks \
 && mkdir -p /app/.timewarrior/data/ \
 && mkdir -p /app/.config/taskwarrior-web/ \
 && chown -R task:task /app && chmod -R 775 /app \
 && systemctl enable cronie.service \
 && cp /usr/share/doc/timew/ext/on-modify.timewarrior /app/.task/hooks/on-modify.timewarrior \
 && chmod +x /app/.task/hooks/on-modify.timewarrior \
 && ( [[ $TASK_ADDON_BUGWARRIOR != "true" ]] || python3 -m pip install --break-system-packages bugwarrior[$TASK_ADDON_BUGWARRIOR_FEATURES]@git+https://github.com/GothenburgBitFactory/bugwarrior.git ) \
 # cleanup
 && pacman --noconfirm -R git python-pip \ 
 && echo "delete orphaned" \ 
 && pacman --noconfirm -Qdtq | pacman --noconfirm -Rs - \
 && echo "clear cache" \
 && pacman --noconfirm -Sc \
 && echo "clean folders" \
 && rm -Rf /var/cache  \
 && rm -Rf /var/log \
 && rm -Rf /var/db \
 && rm -Rf /var/lib \
 && rm -Rf /usr/include \
 && rm -Rf /run

ENV HOME=/app
WORKDIR /app

# Copy files
COPY docker/start.sh /app/bin/start.sh
COPY --from=buildapp /app/dist /app/bin/dist
COPY --from=buildapp /app/target/release/taskwarrior-web /app/bin/taskwarrior-web

RUN chown -R task /app \
    && chmod +x /app/bin/start.sh

USER task

EXPOSE 3000

# Taskwarrior data volume
VOLUME /app/taskdata/
VOLUME /app/.timewarrior/
VOLUME /app/.config/taskwarrior-web/

ENV TASKRC="/app/.taskrc"
ENV TASKDATA="/app/taskdata"
WORKDIR /app

ENTRYPOINT ["/app/bin/start.sh"]
