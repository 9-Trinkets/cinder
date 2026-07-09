FROM node:22-alpine AS builder
WORKDIR /app

COPY cinder-web-ui/package.json cinder-web-ui/package-lock.json ./
RUN npm ci

COPY cinder-web-ui ./
RUN npm run build

FROM nginx:1.27-alpine
WORKDIR /usr/share/nginx/html

COPY cinder-web-ui/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist ./

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
