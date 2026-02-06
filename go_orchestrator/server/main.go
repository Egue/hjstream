package main

import (
	"embed"
	"io/fs"
	"log"
	"net/http"

	"github.com/gin-gonic/gin"
	"mediamtx-orchestrator/internal/api"
	"mediamtx-orchestrator/internal/orchestrator"
)

//go:embed ../../web/static ../../web/templates
var webFiles embed.FS

func main() {
	log.Println("Iniciando MediaMTX Orchestrator...")

	// Inicializar orquestador
	orch := orchestrator.NewOrchestrator()
	defer orch.Shutdown()

	// Configurar Gin en modo release
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()

	// Inicializar API con el orquestador
	api.Initialize(orch)

	// Servir archivos estáticos embebidos
	staticFS, err := fs.Sub(webFiles, "web/static")
	if err != nil {
		log.Fatal(err)
	}
	r.StaticFS("/static", http.FS(staticFS))

	// API endpoints
	apiGroup := r.Group("/api/v1")
	{
		apiGroup.GET("/channels", api.GetChannels)
		apiGroup.POST("/channels", api.CreateChannel)
		apiGroup.DELETE("/channels/:id", api.DeleteChannel)
		apiGroup.GET("/channels/:id/status", api.GetChannelStatus)
		apiGroup.POST("/channels/:id/start", api.StartChannel)
		apiGroup.POST("/channels/:id/stop", api.StopChannel)
	}

	// Página principal
	r.GET("/", func(c *gin.Context) {
		data, err := webFiles.ReadFile("web/templates/index.html")
		if err != nil {
			c.String(http.StatusInternalServerError, "Error loading page")
			return
		}
		c.Data(http.StatusOK, "text/html; charset=utf-8", data)
	})

	log.Println("Servidor iniciado en http://localhost:8080")
	if err := r.Run(":8080"); err != nil {
		log.Fatal("Error iniciando servidor:", err)
	}
}
