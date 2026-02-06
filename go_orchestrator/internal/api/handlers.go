package api

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/yourusername/mediamtx-orchestrator/internal/orchestrator"
)

var orch *orchestrator.Orchestrator

func Initialize(o *orchestrator.Orchestrator) {
	orch = o
}

func GetChannels(c *gin.Context) {
	channels := orch.GetChannelManager().ListChannels()
	c.JSON(http.StatusOK, gin.H{
		"channels": channels,
		"count":    len(channels),
	})
}

func CreateChannel(c *gin.Context) {
	var channel orchestrator.Channel
	if err := c.ShouldBindJSON(&channel); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{
			"error":   "Datos inválidos",
			"details": err.Error(),
		})
		return
	}

	// Validaciones básicas
	if channel.Name == "" {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "El nombre del canal es requerido",
		})
		return
	}

	if channel.RTSPUrl == "" {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "La URL RTSP es requerida",
		})
		return
	}

	if channel.UDPAddress == "" {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "La dirección UDP es requerida",
		})
		return
	}

	if channel.UDPPort <= 0 || channel.UDPPort > 65535 {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "El puerto UDP debe estar entre 1 y 65535",
		})
		return
	}

	// Establecer estado inicial
	channel.Status = "stopped"

	if err := orch.GetChannelManager().AddChannel(&channel); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error": "Error al crear el canal",
		})
		return
	}

	c.JSON(http.StatusCreated, gin.H{
		"message": "Canal creado exitosamente",
		"channel": channel,
	})
}

func DeleteChannel(c *gin.Context) {
	id := c.Param("id")

	// Verificar si el stream está corriendo
	if orch.IsStreamRunning(id) {
		// Detener el stream primero
		if err := orch.StopStream(id); err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{
				"error": "Error deteniendo el stream antes de eliminar",
			})
			return
		}
	}

	if err := orch.GetChannelManager().DeleteChannel(id); err != nil {
		c.JSON(http.StatusNotFound, gin.H{
			"error": "Canal no encontrado",
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"message": "Canal eliminado exitosamente",
	})
}

func GetChannelStatus(c *gin.Context) {
	id := c.Param("id")

	channel, exists := orch.GetChannelManager().GetChannel(id)
	if !exists {
		c.JSON(http.StatusNotFound, gin.H{
			"error": "Canal no encontrado",
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"channel": channel,
		"running": orch.IsStreamRunning(id),
	})
}

func StartChannel(c *gin.Context) {
	id := c.Param("id")

	channel, exists := orch.GetChannelManager().GetChannel(id)
	if !exists {
		c.JSON(http.StatusNotFound, gin.H{
			"error": "Canal no encontrado",
		})
		return
	}

	if err := orch.StartStream(id); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "Error al iniciar el stream",
			"details": err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"message": "Stream iniciado exitosamente",
		"channel": channel,
	})
}

func StopChannel(c *gin.Context) {
	id := c.Param("id")

	channel, exists := orch.GetChannelManager().GetChannel(id)
	if !exists {
		c.JSON(http.StatusNotFound, gin.H{
			"error": "Canal no encontrado",
		})
		return
	}

	if err := orch.StopStream(id); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "Error al detener el stream",
			"details": err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"message": "Stream detenido exitosamente",
		"channel": channel,
	})
}