FROM       rust:1.61-slim as build
WORKDIR    /app
COPY       . .
ENV        HELIX_DISABLE_AUTO_GRAMMAR_BUILD=1
RUN        cargo build

FROM       ubuntu:latest as release
RUN        groupadd helixuser && useradd -ms /bin/bash -g helixuser helixuser
USER       helixuser
WORKDIR    /home/helixuser/.config/helix/runtime
COPY       --from=build --chown=helixuser:helixuser /app/runtime .
WORKDIR    /home/helixuser/helix
COPY       --from=build --chown=helixuser:helixuser /app/target/debug .
RUN        echo "export TERM=xterm-256color" >> ~/.bashrc && \
             echo "export COLORTERM=truecolor" >> ~/.bashrc && \
             echo "export HELIX_RUNTIME=$HOME/.config/helix/runtime" >> ~/.bashrc && \
             echo "export PATH=$HOME/.bin:$PATH" >> ~/.bashrc && \
             chmod 700 hx && \
             mkdir -p ~/.bin && \
             ln -s "$HOME/helix/hx" "$HOME/.bin/hx"
WORKDIR    /home/helixuser/workspace
ENV        PATH=/home/helixuser/.bin:$PATH
ENV        COLORTERM=truecolor
ENTRYPOINT [ "hx", "." ]
