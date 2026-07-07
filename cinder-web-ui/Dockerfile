FROM node:22-alpine
WORKDIR /app

COPY cinder-web-ui/package.json cinder-web-ui/package-lock.json ./
RUN npm ci

COPY cinder-web-ui ./

EXPOSE 5173

CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
