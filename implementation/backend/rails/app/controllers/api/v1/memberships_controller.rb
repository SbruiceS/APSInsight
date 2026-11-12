# frozen_string_literal: true

module Api
  module V1
    class MembershipsController < ApplicationController
      def index
        memberships = Membership.includes(:user, :organization, :project, :role)
        render json: memberships.as_json(include: [:user, :organization, :project, :role])
      end

      def show
        membership = Membership.find(params[:id])
        render json: membership.as_json(include: [:user, :organization, :project, :role])
      end

      def create
        membership = Membership.new(membership_params)
        if membership.save
          AuditEvent.create!(action: "create", resource_type: "Membership", resource_id: membership.id)
          render json: membership, status: :created
        else
          render json: { errors: membership.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def destroy
        membership = Membership.find(params[:id])
        membership.destroy!
        AuditEvent.create!(action: "destroy", resource_type: "Membership", resource_id: membership.id)
        head :no_content
      end

      private

      def membership_params
        params.require(:membership).permit(:user_id, :organization_id, :project_id, :role_id)
      end
    end
  end
end
