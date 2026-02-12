import React from "react"
import type { Metadata, Viewport } from "next"
import { Inter, JetBrains_Mono } from "next/font/google"
import { ThemeProvider } from "@/components/providers/theme-provider"
import { Toaster } from "sonner"

import "./globals.css"

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" })
const jetbrains = JetBrains_Mono({ subsets: ["latin"], variable: "--font-jetbrains" })

export const metadata: Metadata = {
  title: {
    default: "TDLN - Prova criptografica para cada decisao",
    template: "%s | TDLN",
  },
  description:
    "Verificacao criptografica verificavel, offline e sem custodia. Protocolo SIRP para compliance, auditoria e confianca.",
  keywords: [
    "verificacao criptografica",
    "SIRP",
    "compliance",
    "auditoria",
    "bundle offline",
    "prova criptografica",
  ],
  openGraph: {
    title: "TDLN - Prova criptografica para cada decisao",
    description:
      "Gere recibos verificaveis, imutaveis e auditaveis para qualquer decisao. Verificacao offline, sem custodia e sem vendor lock-in.",
    type: "website",
    locale: "pt_BR",
  },
}

export const viewport: Viewport = {
  themeColor: "#0f1117",
  width: "device-width",
  initialScale: 1,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="pt-BR" suppressHydrationWarning>
      <body className={`${inter.variable} ${jetbrains.variable} font-sans antialiased`}>
        <ThemeProvider
          attribute="class"
          defaultTheme="light"
          enableSystem
          disableTransitionOnChange
        >
          {children}
          <Toaster position="top-right" />
        </ThemeProvider>
      </body>
    </html>
  )
}
